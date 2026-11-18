use crate::db::DbState;
use crate::models::{GlossaryEntry, LlmProfile, Project, TranslateProgressEvent, TranslationUnit};
use crate::qa::validate_translation;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ModelsListResponse {
    data: Vec<ModelItem>,
}

#[derive(Deserialize)]
struct ModelItem {
    id: String,
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))
}

fn chat_endpoint_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{}/chat/completions", trimmed)
    }
}

fn models_endpoint_url(base_url: &str) -> String {
    let trimmed = base_url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches("/chat/completions");
    format!("{}/models", trimmed)
}

pub async fn fetch_models(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let client = build_client()?;
    let url = models_endpoint_url(base_url);
    let mut builder = client.get(&url);
    if !api_key.trim().is_empty() {
        builder = builder.bearer_auth(api_key.trim());
    }
    let resp = builder
        .send()
        .await
        .map_err(|e| format!("获取模型列表失败: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), body));
    }
    let parsed: ModelsListResponse =
        serde_json::from_str(&body).map_err(|e| format!("解析模型列表失败: {e}"))?;
    let mut ids: Vec<String> = parsed.data.into_iter().map(|m| m.id).collect();
    ids.sort();
    Ok(ids)
}

pub async fn test_connection(profile: &LlmProfile) -> Result<String, String> {
    let client = build_client()?;
    let url = chat_endpoint_url(&profile.base_url);
    let mut payload = json!({
        "model": profile.model,
        "temperature": 0.0,
        "max_tokens": 32,
        "messages": [
            { "role": "user", "content": "Reply with OK only." }
        ]
    });
    if profile.reasoning_effort != "none" && !profile.reasoning_effort.is_empty() {
        payload["reasoning_effort"] = json!(profile.reasoning_effort);
    }

    let mut builder = client.post(&url).json(&payload);
    if !profile.api_key.trim().is_empty() {
        builder = builder.bearer_auth(profile.api_key.trim());
    }

    let resp = builder
        .send()
        .await
        .map_err(|e| format!("连接失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("HTTP {}: {}", status.as_u16(), text));
    }
    Ok(format!("连接成功 ({})", profile.model))
}

fn build_system_prompt(project: &Project, glossary: &[GlossaryEntry]) -> String {
    let mut lines = vec![
        "你是一位资深的软件本地化与汉化专家。请将提供的软件界面文本精准翻译为简体中文。".to_string(),
        format!("目标软件：{} {}", project.software, project.version),
    ];
    if !project.description.trim().is_empty() {
        lines.push(format!("软件背景：{}", project.description.trim()));
    }
    lines.push("严格遵守以下规则：".into());
    lines.push("1. 必须原样保留所有占位符和格式标签（如 %s, %d, {0}, {name}, {{var}}, ${var}, \\n, <b>...</b> 等），不得增删或翻译占位符内部变量名。".into());
    lines.push("2. 界面按钮、菜单项译文应简洁地道（符合 Windows/macOS 中文软件习惯）。".into());
    lines.push("3. 仅返回合法 JSON 对象，格式为 {\"translations\": [{\"id\": \"...\", \"target\": \"...\"}]}，不要输出任何额外解释或 Markdown 代码块之外的废话。".into());

    if !glossary.is_empty() {
        lines.push("\n必须严格遵循的术语表：".into());
        for g in glossary.iter().take(80) {
            if g.do_not_translate {
                lines.push(format!("- \"{}\" -> 保持原文不翻译", g.source));
            } else {
                lines.push(format!("- \"{}\" -> \"{}\"", g.source, g.target));
            }
        }
    }
    lines.join("\n")
}

#[derive(Serialize)]
struct InputItem<'a> {
    id: &'a str,
    key: &'a str,
    source: &'a str,
}

#[derive(Deserialize)]
struct OutputWrapper {
    translations: Vec<OutputItem>,
}

#[derive(Deserialize)]
struct OutputItem {
    id: String,
    target: String,
}

fn strip_code_fences(s: &str) -> &str {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```json") {
        rest.trim().strip_suffix("```").unwrap_or(rest).trim()
    } else if let Some(rest) = t.strip_prefix("```") {
        rest.trim().strip_suffix("```").unwrap_or(rest).trim()
    } else {
        t
    }
}

async fn translate_batch_request(
    client: &Client,
    profile: &LlmProfile,
    sys_prompt: &str,
    batch: &[TranslationUnit],
    reference_image_data_url: Option<&str>,
) -> Result<Vec<(String, String)>, String> {
    let items: Vec<InputItem> = batch
        .iter()
        .map(|u| InputItem {
            id: &u.id,
            key: &u.key_path,
            source: &u.source,
        })
        .collect();
    let user_json = serde_json::to_string(&items).map_err(|e| e.to_string())?;
    let prompt_text = format!("请翻译以下 JSON 数组中的每一项：\n{}", user_json);

    let user_content: Value = if let (true, Some(img_url)) = (profile.supports_vision, reference_image_data_url) {
        json!([
            { "type": "text", "text": format!("以下是软件界面参考截图，请结合界面上下文翻译以下 JSON 数组：\n{}", user_json) },
            { "type": "image_url", "image_url": { "url": img_url } }
        ])
    } else {
        json!(prompt_text)
    };

    let mut payload = json!({
        "model": profile.model,
        "temperature": profile.temperature,
        "messages": [
            { "role": "system", "content": sys_prompt },
            { "role": "user", "content": user_content }
        ]
    });

    if profile.max_tokens > 0 {
        payload["max_tokens"] = json!(profile.max_tokens);
    }
    if profile.reasoning_effort != "none" && !profile.reasoning_effort.is_empty() {
        payload["reasoning_effort"] = json!(profile.reasoning_effort);
    }
    if profile.provider != "ollama" {
        payload["response_format"] = json!({ "type": "json_object" });
    }

    let url = chat_endpoint_url(&profile.base_url);
    let max_attempts = (profile.max_retries.max(0) as u32).min(5) + 1;
    let mut last_err = String::new();

    for attempt in 0..max_attempts {
        if attempt > 0 {
            let delay_ms = 600 * (1u64 << (attempt - 1));
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        let mut builder = client.post(&url).json(&payload);
        if !profile.api_key.trim().is_empty() {
            builder = builder.bearer_auth(profile.api_key.trim());
        }

        match builder.send().await {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if status.as_u16() == 429 || status.is_server_error() {
                    last_err = format!("API 返回 {}: {}", status.as_u16(), body);
                    continue;
                }
                if !status.is_success() {
                    return Err(format!("API 返回 {}: {}", status.as_u16(), body));
                }
                let parsed: ChatResponse = serde_json::from_str(&body)
                    .map_err(|e| format!("解析 API 响应失败: {e}"))?;
                let content = parsed
                    .choices
                    .first()
                    .and_then(|c| c.message.content.as_deref())
                    .unwrap_or("");
                let clean = strip_code_fences(content);

                if let Ok(wrapper) = serde_json::from_str::<OutputWrapper>(clean) {
                    return Ok(wrapper
                        .translations
                        .into_iter()
                        .map(|x| (x.id, x.target))
                        .collect());
                }
                if let Ok(arr) = serde_json::from_str::<Vec<OutputItem>>(clean) {
                    return Ok(arr.into_iter().map(|x| (x.id, x.target)).collect());
                }
                return Err(format!(
                    "模型未返回约定的 JSON 结构: {}",
                    clean.chars().take(180).collect::<String>()
                ));
            }
            Err(e) => {
                last_err = format!("请求超时或网络错误: {e}");
            }
        }
    }

    Err(last_err)
}

pub async fn run_project_translation(
    app: &AppHandle,
    db: &DbState,
    project_id: &str,
    file_id: Option<&str>,
    profile_id: &str,
    only_untranslated: bool,
    reference_image_data_url: Option<String>,
) -> Result<(usize, usize, usize), String> {
    let project = db
        .get_project(project_id)?
        .ok_or_else(|| "项目不存在".to_string())?;
    let profile = db
        .list_llm_profiles()?
        .into_iter()
        .find(|p| p.id == profile_id)
        .ok_or_else(|| "未找到指定的 LLM 模型配置".to_string())?;

    if profile.provider != "ollama" && profile.api_key.trim().is_empty() {
        return Err(format!("请先在「AI 模型配置」中为 {} 填写 API Key", profile.name));
    }

    let glossary = db.list_glossary(Some(project_id))?;
    let memory = db.list_memory()?;
    let mem_map: HashMap<String, String> = memory
        .into_iter()
        .map(|m| (m.source.trim().to_ascii_lowercase(), m.target))
        .collect();

    let all_units = db.list_units(project_id, file_id)?;
    let target_units: Vec<TranslationUnit> = all_units
        .into_iter()
        .filter(|u| {
            if only_untranslated {
                u.target.trim().is_empty() || u.status == "untranslated"
            } else {
                u.status != "reviewed"
            }
        })
        .collect();

    let total = target_units.len();
    if total == 0 {
        return Ok((0, 0, 0));
    }

    let mut mem_updates = Vec::new();
    let mut llm_queue = Vec::new();
    for u in target_units {
        let key = u.source.trim().to_ascii_lowercase();
        if let Some(hit) = mem_map.get(&key) {
            let qa = validate_translation(&u.source, hit);
            let hits = matching_glossary_ids(&u.source, &glossary);
            mem_updates.push((u.id, hit.clone(), "machine".to_string(), qa, hits, 100));
        } else {
            llm_queue.push(u);
        }
    }

    let mut completed = mem_updates.len();
    let mut succeeded = mem_updates.len();
    let mut failed = 0usize;

    if !mem_updates.is_empty() {
        db.batch_apply_translations(project_id, &mem_updates)?;
    }

    let _ = app.emit(
        "translate:progress",
        TranslateProgressEvent {
            project_id: project_id.to_string(),
            total,
            completed,
            succeeded,
            failed,
            message: format!(
                "翻译记忆命中 {} 条，正在并发调用 {}...",
                mem_updates.len(),
                profile.model
            ),
            done: llm_queue.is_empty(),
        },
    );

    if llm_queue.is_empty() {
        return Ok((total, succeeded, failed));
    }

    let batch_size = (profile.batch_size.max(5) as usize).min(60);
    let concurrency = (profile.concurrency.max(1) as usize).min(12);
    let sem = Arc::new(Semaphore::new(concurrency));
    let client = Arc::new(build_client()?);
    let sys_prompt = Arc::new(build_system_prompt(&project, &glossary));
    let profile_arc = Arc::new(profile);
    let img_arc = Arc::new(reference_image_data_url);

    let chunks: Vec<Vec<TranslationUnit>> = llm_queue
        .chunks(batch_size)
        .map(|c| c.to_vec())
        .collect();

    let mut join_set = tokio::task::JoinSet::new();
    for chunk in chunks {
        let sem = Arc::clone(&sem);
        let client = Arc::clone(&client);
        let sys_prompt = Arc::clone(&sys_prompt);
        let profile = Arc::clone(&profile_arc);
        let img = Arc::clone(&img_arc);
        join_set.spawn(async move {
            let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
            let res = translate_batch_request(
                &client,
                &profile,
                &sys_prompt,
                &chunk,
                img.as_deref(),
            )
            .await;
            Ok::<(Vec<TranslationUnit>, Result<Vec<(String, String)>, String>), String>((chunk, res))
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok((chunk, Ok(pairs)))) => {
                let mut pair_map: HashMap<String, String> = pairs.into_iter().collect();
                let mut batch_updates = Vec::new();
                for u in &chunk {
                    if let Some(target) = pair_map.remove(&u.id) {
                        let qa = validate_translation(&u.source, &target);
                        let hits = matching_glossary_ids(&u.source, &glossary);
                        batch_updates.push((
                            u.id.clone(),
                            target,
                            "machine".to_string(),
                            qa,
                            hits,
                            90,
                        ));
                        succeeded += 1;
                    } else {
                        failed += 1;
                    }
                    completed += 1;
                }
                if !batch_updates.is_empty() {
                    db.batch_apply_translations(project_id, &batch_updates)?;
                }
                let _ = app.emit(
                    "translate:progress",
                    TranslateProgressEvent {
                        project_id: project_id.to_string(),
                        total,
                        completed,
                        succeeded,
                        failed,
                        message: format!(
                            "已完成 {}/{} 条（成功 {}，失败 {}）",
                            completed, total, succeeded, failed
                        ),
                        done: false,
                    },
                );
            }
            Ok(Ok((chunk, Err(err)))) => {
                failed += chunk.len();
                completed += chunk.len();
                let _ = app.emit(
                    "translate:progress",
                    TranslateProgressEvent {
                        project_id: project_id.to_string(),
                        total,
                        completed,
                        succeeded,
                        failed,
                        message: format!("批次失败: {}", err),
                        done: false,
                    },
                );
            }
            _ => {}
        }
    }

    let _ = app.emit(
        "translate:progress",
        TranslateProgressEvent {
            project_id: project_id.to_string(),
            total,
            completed,
            succeeded,
            failed,
            message: format!(
                "翻译完成：共 {} 条，成功 {} 条，失败 {} 条",
                total, succeeded, failed
            ),
            done: true,
        },
    );

    Ok((total, succeeded, failed))
}

fn matching_glossary_ids(source: &str, glossary: &[GlossaryEntry]) -> Vec<String> {
    let lower = source.to_ascii_lowercase();
    glossary
        .iter()
        .filter(|g| !g.source.is_empty() && lower.contains(&g.source.to_ascii_lowercase()))
        .map(|g| g.id.clone())
        .collect()
}
