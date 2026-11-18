use crate::models::{
    DashboardStats, GlossaryEntry, LlmProfile, PatchBuild, Project, ResourceFile,
    TranslationMemory, TranslationUnit,
};
use crate::parsers::{parse_content, ParsedEntry};
use crate::qa::validate_translation;
use crate::scanner::sha256_hex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

pub struct DbState {
    pub conn: Mutex<Connection>,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn init_db(db_path: &Path) -> Result<DbState, String> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(db_path).map_err(|e| format!("打开数据库失败: {e}"))?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            software TEXT NOT NULL,
            version TEXT NOT NULL,
            source_dir TEXT,
            exe_name TEXT,
            default_paths_json TEXT NOT NULL DEFAULT '[]',
            description TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'draft',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS resource_files (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            rel_path TEXT NOT NULL,
            target_rel_path TEXT NOT NULL,
            format TEXT NOT NULL,
            original_sha256 TEXT NOT NULL,
            raw_content TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_files_project ON resource_files(project_id);

        CREATE TABLE IF NOT EXISTS translation_units (
            id TEXT PRIMARY KEY,
            file_id TEXT NOT NULL REFERENCES resource_files(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            key_path TEXT NOT NULL,
            source TEXT NOT NULL,
            target TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'untranslated',
            qa_warning TEXT,
            term_hits_json TEXT NOT NULL DEFAULT '[]',
            fuzzy_score INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_units_project ON translation_units(project_id);
        CREATE INDEX IF NOT EXISTS idx_units_file ON translation_units(file_id);
        CREATE INDEX IF NOT EXISTS idx_units_status ON translation_units(project_id, status);

        CREATE TABLE IF NOT EXISTS glossary_entries (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            source TEXT NOT NULL,
            target TEXT NOT NULL,
            domain TEXT NOT NULL DEFAULT 'general',
            context TEXT NOT NULL DEFAULT '',
            do_not_translate INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS translation_memory (
            id TEXT PRIMARY KEY,
            source TEXT NOT NULL UNIQUE,
            target TEXT NOT NULL,
            domain TEXT NOT NULL DEFAULT 'general',
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS llm_profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            provider TEXT NOT NULL,
            base_url TEXT NOT NULL,
            api_key TEXT NOT NULL,
            model TEXT NOT NULL,
            concurrency INTEGER NOT NULL DEFAULT 4,
            batch_size INTEGER NOT NULL DEFAULT 25,
            temperature REAL NOT NULL DEFAULT 0.2,
            is_default INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS patch_builds (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            version TEXT NOT NULL,
            build_type TEXT NOT NULL,
            output_path TEXT NOT NULL,
            file_count INTEGER NOT NULL,
            unit_count INTEGER NOT NULL,
            note TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL
        );
        ",
    )
    .map_err(|e| format!("初始化表结构失败: {e}"))?;

    seed_defaults(&conn)?;
    Ok(DbState {
        conn: Mutex::new(conn),
    })
}

fn seed_defaults(conn: &Connection) -> Result<(), String> {
    let llm_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM llm_profiles", [], |r| r.get(0))
        .unwrap_or(0);
    if llm_count == 0 {
        let presets = [
            (
                "llm-deepseek",
                "DeepSeek V3 (推荐)",
                "deepseek",
                "https://api.deepseek.com/v1",
                "",
                "deepseek-chat",
                4,
                25,
                0.2,
                1,
            ),
            (
                "llm-qwen",
                "通义千问 Qwen-Plus",
                "qwen",
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "",
                "qwen-plus",
                4,
                25,
                0.2,
                0,
            ),
            (
                "llm-ollama",
                "本地 Ollama (离线)",
                "ollama",
                "http://localhost:11434/v1",
                "ollama",
                "qwen2.5:7b",
                2,
                15,
                0.2,
                0,
            ),
        ];
        for p in presets {
            conn.execute(
                "INSERT INTO llm_profiles (id, name, provider, base_url, api_key, model, concurrency, batch_size, temperature, is_default)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7, p.8, p.9],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let gl_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM glossary_entries", [], |r| r.get(0))
        .unwrap_or(0);
    if gl_count == 0 {
        let terms = [
            ("gl-1", "global", "Settings", "设置", "ui", "主菜单或按钮", 0),
            ("gl-2", "global", "Preferences", "首选项", "ui", "配置菜单", 0),
            ("gl-3", "global", "Workspace", "工作区", "ide", "编辑器工作空间", 0),
            ("gl-4", "global", "Extension", "扩展", "ide", "插件/扩展", 0),
            ("gl-5", "global", "API", "API", "tech", "保留英文缩写", 1),
        ];
        for t in terms {
            conn.execute(
                "INSERT INTO glossary_entries (id, project_id, source, target, domain, context, do_not_translate)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![t.0, t.1, t.2, t.3, t.4, t.5, t.6],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

impl DbState {
    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT p.id, p.name, p.software, p.version, p.source_dir, p.exe_name,
                        p.default_paths_json, p.description, p.status, p.created_at, p.updated_at,
                        (SELECT COUNT(*) FROM translation_units u WHERE u.project_id = p.id) AS total_units,
                        (SELECT COUNT(*) FROM translation_units u WHERE u.project_id = p.id AND TRIM(u.target) != '') AS translated_units,
                        (SELECT COUNT(*) FROM translation_units u WHERE u.project_id = p.id AND u.status = 'reviewed') AS reviewed_units
                 FROM projects p
                 ORDER BY p.updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let paths_json: String = r.get(6)?;
                let default_paths: Vec<String> =
                    serde_json::from_str(&paths_json).unwrap_or_default();
                Ok(Project {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    software: r.get(2)?,
                    version: r.get(3)?,
                    source_dir: r.get(4)?,
                    exe_name: r.get(5)?,
                    default_paths,
                    description: r.get(7)?,
                    status: r.get(8)?,
                    created_at: r.get(9)?,
                    updated_at: r.get(10)?,
                    total_units: r.get(11)?,
                    translated_units: r.get(12)?,
                    reviewed_units: r.get(13)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn get_project(&self, id: &str) -> Result<Option<Project>, String> {
        let list = self.list_projects()?;
        Ok(list.into_iter().find(|p| p.id == id))
    }

    pub fn create_project(
        &self,
        name: String,
        software: String,
        version: String,
        source_dir: Option<String>,
        exe_name: Option<String>,
        default_paths: Vec<String>,
        description: String,
    ) -> Result<Project, String> {
        let id = format!("proj_{}", &Uuid::new_v4().simple().to_string()[..10]);
        let now = now_ms();
        let paths_json = serde_json::to_string(&default_paths).unwrap_or_else(|_| "[]".into());
        {
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "INSERT INTO projects (id, name, software, version, source_dir, exe_name, default_paths_json, description, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'draft', ?9, ?10)",
                params![id, name, software, version, source_dir, exe_name, paths_json, description, now, now],
            )
            .map_err(|e| e.to_string())?;
        }
        self.get_project(&id)?
            .ok_or_else(|| "创建项目后查询失败".into())
    }

    pub fn update_project_meta(
        &self,
        id: &str,
        name: &str,
        software: &str,
        version: &str,
        exe_name: Option<&str>,
        default_paths: &[String],
        description: &str,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let paths_json = serde_json::to_string(default_paths).unwrap_or_else(|_| "[]".into());
        conn.execute(
            "UPDATE projects SET name=?1, software=?2, version=?3, exe_name=?4, default_paths_json=?5, description=?6, updated_at=?7 WHERE id=?8",
            params![name, software, version, exe_name, paths_json, description, now_ms(), id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn add_resource_file(
        &self,
        project_id: &str,
        rel_path: &str,
        target_rel_path: &str,
        format: &str,
        raw_content: &str,
    ) -> Result<ResourceFile, String> {
        let parsed: Vec<ParsedEntry> = parse_content(raw_content, format);
        let file_id = format!("file_{}", &Uuid::new_v4().simple().to_string()[..10]);
        let sha = sha256_hex(raw_content.as_bytes());
        let now = now_ms();

        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO resource_files (id, project_id, rel_path, target_rel_path, format, original_sha256, raw_content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![file_id, project_id, rel_path, target_rel_path, format, sha, raw_content],
        )
        .map_err(|e| e.to_string())?;

        let mut translated_count = 0i64;
        for entry in &parsed {
            let uid = format!("u_{}", &Uuid::new_v4().simple().to_string()[..12]);
            let has_existing = !entry.existing_target.trim().is_empty()
                && entry.existing_target != entry.source;
            let status = if has_existing {
                translated_count += 1;
                "translated"
            } else {
                "untranslated"
            };
            let qa_warn = if has_existing {
                validate_translation(&entry.source, &entry.existing_target)
            } else {
                None
            };
            tx.execute(
                "INSERT INTO translation_units (id, file_id, project_id, key_path, source, target, status, qa_warning, term_hits_json, fuzzy_score, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '[]', 0, ?9)",
                params![uid, file_id, project_id, entry.key_path, entry.source, entry.existing_target, status, qa_warn, now],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "UPDATE projects SET status = CASE WHEN status = 'draft' THEN 'translating' ELSE status END, updated_at = ?1 WHERE id = ?2",
            params![now, project_id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;

        Ok(ResourceFile {
            id: file_id,
            project_id: project_id.to_string(),
            rel_path: rel_path.to_string(),
            target_rel_path: target_rel_path.to_string(),
            format: format.to_string(),
            original_sha256: sha,
            unit_count: parsed.len() as i64,
            translated_count,
            raw_content: raw_content.to_string(),
        })
    }

    pub fn list_files(&self, project_id: &str) -> Result<Vec<ResourceFile>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT f.id, f.project_id, f.rel_path, f.target_rel_path, f.format, f.original_sha256, f.raw_content,
                        (SELECT COUNT(*) FROM translation_units u WHERE u.file_id = f.id) AS unit_count,
                        (SELECT COUNT(*) FROM translation_units u WHERE u.file_id = f.id AND TRIM(u.target) != '') AS translated_count
                 FROM resource_files f
                 WHERE f.project_id = ?1
                 ORDER BY f.rel_path ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |r| {
                Ok(ResourceFile {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    rel_path: r.get(2)?,
                    target_rel_path: r.get(3)?,
                    format: r.get(4)?,
                    original_sha256: r.get(5)?,
                    raw_content: r.get(6)?,
                    unit_count: r.get(7)?,
                    translated_count: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn update_file_target_path(&self, file_id: &str, target_rel_path: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE resource_files SET target_rel_path = ?1 WHERE id = ?2",
            params![target_rel_path, file_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_units(&self, project_id: &str, file_id: Option<&str>) -> Result<Vec<TranslationUnit>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = if file_id.is_some() {
            "SELECT id, file_id, project_id, key_path, source, target, status, qa_warning, term_hits_json, fuzzy_score, updated_at
             FROM translation_units WHERE project_id = ?1 AND file_id = ?2 ORDER BY rowid ASC"
        } else {
            "SELECT id, file_id, project_id, key_path, source, target, status, qa_warning, term_hits_json, fuzzy_score, updated_at
             FROM translation_units WHERE project_id = ?1 ORDER BY rowid ASC"
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let map_row = |r: &rusqlite::Row<'_>| -> rusqlite::Result<TranslationUnit> {
            let th_json: String = r.get(8)?;
            let term_hits: Vec<String> = serde_json::from_str(&th_json).unwrap_or_default();
            Ok(TranslationUnit {
                id: r.get(0)?,
                file_id: r.get(1)?,
                project_id: r.get(2)?,
                key_path: r.get(3)?,
                source: r.get(4)?,
                target: r.get(5)?,
                status: r.get(6)?,
                qa_warning: r.get(7)?,
                term_hits,
                fuzzy_score: r.get(9)?,
                updated_at: r.get(10)?,
            })
        };
        let rows = if let Some(fid) = file_id {
            stmt.query_map(params![project_id, fid], map_row)
        } else {
            stmt.query_map(params![project_id], map_row)
        }
        .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn update_unit(&self, id: &str, target: &str, status: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let (source, project_id): (String, String) = conn
            .query_row(
                "SELECT source, project_id FROM translation_units WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;

        let qa = validate_translation(&source, target);
        let now = now_ms();
        conn.execute(
            "UPDATE translation_units SET target = ?1, status = ?2, qa_warning = ?3, updated_at = ?4 WHERE id = ?5",
            params![target, status, qa, now, id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![now, project_id],
        )
        .map_err(|e| e.to_string())?;

        if !target.trim().is_empty() && (status == "translated" || status == "reviewed") {
            let tm_id = format!("tm_{}", &Uuid::new_v4().simple().to_string()[..10]);
            let _ = conn.execute(
                "INSERT INTO translation_memory (id, source, target, domain, updated_at)
                 VALUES (?1, ?2, ?3, 'general', ?4)
                 ON CONFLICT(source) DO UPDATE SET target = excluded.target, updated_at = excluded.updated_at",
                params![tm_id, source.trim(), target.trim(), now],
            );
        }

        Ok(qa)
    }

    pub fn batch_apply_translations(
        &self,
        project_id: &str,
        updates: &[(String, String, String, Option<String>, Vec<String>, i64)],
    ) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        let now = now_ms();
        for (unit_id, target, status, qa_warn, term_hits, fuzzy) in updates {
            let th_json = serde_json::to_string(term_hits).unwrap_or_else(|_| "[]".into());
            tx.execute(
                "UPDATE translation_units SET target = ?1, status = ?2, qa_warning = ?3, term_hits_json = ?4, fuzzy_score = ?5, updated_at = ?6 WHERE id = ?7",
                params![target, status, qa_warn, th_json, fuzzy, now, unit_id],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "UPDATE projects SET updated_at = ?1 WHERE id = ?2",
            params![now, project_id],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_glossary(&self, project_id: Option<&str>) -> Result<Vec<GlossaryEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, source, target, domain, context, do_not_translate
                 FROM glossary_entries
                 WHERE (?1 IS NULL OR project_id = ?1 OR project_id = 'global')
                 ORDER BY source ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |r| {
                let dnt: i64 = r.get(6)?;
                Ok(GlossaryEntry {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    source: r.get(2)?,
                    target: r.get(3)?,
                    domain: r.get(4)?,
                    context: r.get(5)?,
                    do_not_translate: dnt != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn upsert_glossary(&self, entry: GlossaryEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let id = if entry.id.is_empty() {
            format!("gl_{}", &Uuid::new_v4().simple().to_string()[..10])
        } else {
            entry.id
        };
        conn.execute(
            "INSERT INTO glossary_entries (id, project_id, source, target, domain, context, do_not_translate)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               project_id = excluded.project_id,
               source = excluded.source,
               target = excluded.target,
               domain = excluded.domain,
               context = excluded.context,
               do_not_translate = excluded.do_not_translate",
            params![
                id,
                entry.project_id,
                entry.source,
                entry.target,
                entry.domain,
                entry.context,
                if entry.do_not_translate { 1 } else { 0 }
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_glossary(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM glossary_entries WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_memory(&self) -> Result<Vec<TranslationMemory>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, source, target, domain, updated_at FROM translation_memory ORDER BY updated_at DESC LIMIT 2000")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(TranslationMemory {
                    id: r.get(0)?,
                    source: r.get(1)?,
                    target: r.get(2)?,
                    domain: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn list_llm_profiles(&self) -> Result<Vec<LlmProfile>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, provider, base_url, api_key, model, concurrency, batch_size, temperature, is_default FROM llm_profiles")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                let is_def: i64 = r.get(9)?;
                Ok(LlmProfile {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    provider: r.get(2)?,
                    base_url: r.get(3)?,
                    api_key: r.get(4)?,
                    model: r.get(5)?,
                    concurrency: r.get(6)?,
                    batch_size: r.get(7)?,
                    temperature: r.get(8)?,
                    is_default: is_def != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn upsert_llm_profile(&self, p: LlmProfile) -> Result<(), String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        if p.is_default {
            tx.execute("UPDATE llm_profiles SET is_default = 0", [])
                .map_err(|e| e.to_string())?;
        }
        let id = if p.id.is_empty() {
            format!("llm_{}", &Uuid::new_v4().simple().to_string()[..8])
        } else {
            p.id
        };
        tx.execute(
            "INSERT INTO llm_profiles (id, name, provider, base_url, api_key, model, concurrency, batch_size, temperature, is_default)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               provider = excluded.provider,
               base_url = excluded.base_url,
               api_key = excluded.api_key,
               model = excluded.model,
               concurrency = excluded.concurrency,
               batch_size = excluded.batch_size,
               temperature = excluded.temperature,
               is_default = excluded.is_default",
            params![
                id,
                p.name,
                p.provider,
                p.base_url,
                p.api_key,
                p.model,
                p.concurrency,
                p.batch_size,
                p.temperature,
                if p.is_default { 1 } else { 0 }
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_llm_profile(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM llm_profiles WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn record_patch_build(&self, b: PatchBuild) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO patch_builds (id, project_id, version, build_type, output_path, file_count, unit_count, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![b.id, b.project_id, b.version, b.build_type, b.output_path, b.file_count, b.unit_count, b.note, b.created_at],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE projects SET status = 'published', version = ?1, updated_at = ?2 WHERE id = ?3",
            params![b.version, now_ms(), b.project_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_patch_builds(&self, project_id: &str) -> Result<Vec<PatchBuild>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, version, build_type, output_path, file_count, unit_count, note, created_at
                 FROM patch_builds WHERE project_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![project_id], |r| {
                Ok(PatchBuild {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    version: r.get(2)?,
                    build_type: r.get(3)?,
                    output_path: r.get(4)?,
                    file_count: r.get(5)?,
                    unit_count: r.get(6)?,
                    note: r.get(7)?,
                    created_at: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn get_dashboard_stats(&self) -> Result<DashboardStats, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        Ok(DashboardStats {
            project_count: conn
                .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
                .unwrap_or(0),
            total_units: conn
                .query_row("SELECT COUNT(*) FROM translation_units", [], |r| r.get(0))
                .unwrap_or(0),
            translated_units: conn
                .query_row(
                    "SELECT COUNT(*) FROM translation_units WHERE TRIM(target) != ''",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0),
            reviewed_units: conn
                .query_row(
                    "SELECT COUNT(*) FROM translation_units WHERE status = 'reviewed'",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0),
            qa_warning_units: conn
                .query_row(
                    "SELECT COUNT(*) FROM translation_units WHERE qa_warning IS NOT NULL",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or(0),
            patch_count: conn
                .query_row("SELECT COUNT(*) FROM patch_builds", [], |r| r.get(0))
                .unwrap_or(0),
            glossary_count: conn
                .query_row("SELECT COUNT(*) FROM glossary_entries", [], |r| r.get(0))
                .unwrap_or(0),
            memory_count: conn
                .query_row("SELECT COUNT(*) FROM translation_memory", [], |r| r.get(0))
                .unwrap_or(0),
        })
    }
}
