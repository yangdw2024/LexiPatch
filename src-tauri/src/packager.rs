use crate::db::DbState;
use crate::models::{PatchBuild, Project, ResourceFile, TranslationUnit};
use crate::parsers::patch_content;
use crate::scanner::sha256_hex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const MAGIC: &[u8; 8] = b"TCPATCH2";

#[derive(Debug, Serialize, Deserialize)]
struct PatchFileEntry {
    target_path: String,
    original_sha256: Option<String>,
    patched_sha256: String,
    unit_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct PatchManifest {
    patch_id: String,
    project_name: String,
    software_name: String,
    version: String,
    author: String,
    note: String,
    exe_name: Option<String>,
    default_paths: Vec<String>,
    created_at: String,
    files: Vec<PatchFileEntry>,
}

fn locate_patcher_stub(app: &AppHandle) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Ok(res_dir) = app.path().resource_dir() {
        candidates.push(res_dir.join("patcher-stub.exe"));
        candidates.push(res_dir.join("resources").join("patcher-stub.exe"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("patcher-stub.exe"));
            candidates.push(dir.join("resources").join("patcher-stub.exe"));
            // Dev mode: target/debug/transcore-app.exe -> target/release/transcore-patcher.exe
            if let Some(target_dir) = dir.parent() {
                candidates.push(target_dir.join("release").join("transcore-patcher.exe"));
                candidates.push(target_dir.join("debug").join("transcore-patcher.exe"));
            }
        }
    }
    // Workspace relative fallback
    candidates.push(PathBuf::from("src-tauri/resources/patcher-stub.exe"));
    candidates.push(PathBuf::from("target/release/transcore-patcher.exe"));

    for c in candidates {
        if c.is_file() {
            return Ok(c);
        }
    }
    Err("未找到独立补丁引导器 (patcher-stub.exe)，请先编译 patcher-stub。".into())
}

fn group_units(units: &[TranslationUnit]) -> HashMap<String, Vec<TranslationUnit>> {
    let mut map: HashMap<String, Vec<TranslationUnit>> = HashMap::new();
    for u in units {
        map.entry(u.file_id.clone()).or_default().push(u.clone());
    }
    map
}

pub fn build_exe_patcher(
    app: &AppHandle,
    db: &DbState,
    project_id: &str,
    version: &str,
    author: &str,
    note: &str,
    output_path: &str,
) -> Result<PatchBuild, String> {
    let project: Project = db
        .get_project(project_id)?
        .ok_or_else(|| "项目不存在".to_string())?;
    let files: Vec<ResourceFile> = db.list_files(project_id)?;
    if files.is_empty() {
        return Err("当前项目没有任何资源文件，无法生成补丁。".into());
    }
    let units = db.list_units(project_id, None)?;
    let by_file = group_units(&units);

    let stub_path = locate_patcher_stub(app)?;
    let stub_bytes = fs::read(&stub_path).map_err(|e| format!("读取补丁引导器失败: {e}"))?;

    let patch_id = format!("patch_{}", &Uuid::new_v4().simple().to_string()[..10]);
    let created_at_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut zip_buf = Cursor::new(Vec::new());
    let mut manifest_files = Vec::new();
    {
        let mut zip = ZipWriter::new(&mut zip_buf);
        let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for file in &files {
            let file_units = by_file.get(&file.id).map(|v| v.as_slice()).unwrap_or(&[]);
            let patched_str = patch_content(&file.raw_content, &file.format, file_units);
            let patched_bytes = patched_str.as_bytes();
            let patched_hash = sha256_hex(patched_bytes);

            let target_norm = file.target_rel_path.replace('\\', "/");
            let zip_entry_name = format!("files/{}", target_norm);
            zip.start_file(&zip_entry_name, opts)
                .map_err(|e| e.to_string())?;
            zip.write_all(patched_bytes).map_err(|e| e.to_string())?;

            manifest_files.push(PatchFileEntry {
                target_path: target_norm,
                original_sha256: Some(file.original_sha256.clone()),
                patched_sha256: patched_hash,
                unit_count: file_units.len(),
            });
        }

        let manifest = PatchManifest {
            patch_id: patch_id.clone(),
            project_name: project.name.clone(),
            software_name: project.software.clone(),
            version: version.to_string(),
            author: author.to_string(),
            note: note.to_string(),
            exe_name: project.exe_name.clone(),
            default_paths: project.default_paths.clone(),
            created_at: created_at_str,
            files: manifest_files,
        };
        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
        zip.start_file("patch_manifest.json", opts)
            .map_err(|e| e.to_string())?;
        zip.write_all(manifest_json.as_bytes())
            .map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
    }

    let payload = zip_buf.into_inner();
    let payload_len = payload.len() as u64;

    let out_p = Path::new(output_path);
    if let Some(parent) = out_p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut out_file = fs::File::create(out_p).map_err(|e| format!("写入 .exe 失败: {e}"))?;
    out_file.write_all(&stub_bytes).map_err(|e| e.to_string())?;
    out_file.write_all(&payload).map_err(|e| e.to_string())?;
    out_file
        .write_all(&payload_len.to_le_bytes())
        .map_err(|e| e.to_string())?;
    out_file.write_all(MAGIC).map_err(|e| e.to_string())?;

    let build = PatchBuild {
        id: patch_id,
        project_id: project_id.to_string(),
        version: version.to_string(),
        build_type: "exe".into(),
        output_path: output_path.to_string(),
        file_count: files.len() as i64,
        unit_count: units.len() as i64,
        note: note.to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    db.record_patch_build(build.clone())?;
    Ok(build)
}

pub fn build_zip_package(
    db: &DbState,
    project_id: &str,
    version: &str,
    note: &str,
    output_path: &str,
) -> Result<PatchBuild, String> {
    let project: Project = db
        .get_project(project_id)?
        .ok_or_else(|| "项目不存在".to_string())?;
    let files = db.list_files(project_id)?;
    if files.is_empty() {
        return Err("当前项目没有任何资源文件，无法打包。".into());
    }
    let units = db.list_units(project_id, None)?;
    let by_file = group_units(&units);

    let out_p = Path::new(output_path);
    if let Some(parent) = out_p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let f = fs::File::create(out_p).map_err(|e| format!("创建 ZIP 失败: {e}"))?;
    let mut zip = ZipWriter::new(f);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for file in &files {
        let file_units = by_file.get(&file.id).map(|v| v.as_slice()).unwrap_or(&[]);
        let patched_str = patch_content(&file.raw_content, &file.format, file_units);
        let entry_path = format!("localized/{}", file.target_rel_path.replace('\\', "/"));
        zip.start_file(&entry_path, opts).map_err(|e| e.to_string())?;
        zip.write_all(patched_str.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    let translated = units.iter().filter(|u| !u.target.trim().is_empty()).count();
    let readme = format!(
        "{} ({}) 汉化包\n版本: {}\n文件数: {}\n词条数: {} (已翻译 {})\n说明: {}\n\n将 localized/ 目录内文件覆盖至软件安装目录即可。\n由 译铸 LexiPatch v2.0 生成。",
        project.name, project.software, version, files.len(), units.len(), translated, note
    );
    zip.start_file("README.txt", opts).map_err(|e| e.to_string())?;
    zip.write_all(readme.as_bytes()).map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;

    let build = PatchBuild {
        id: format!("zip_{}", &Uuid::new_v4().simple().to_string()[..10]),
        project_id: project_id.to_string(),
        version: version.to_string(),
        build_type: "zip".into(),
        output_path: output_path.to_string(),
        file_count: files.len() as i64,
        unit_count: units.len() as i64,
        note: note.to_string(),
        created_at: chrono::Utc::now().timestamp_millis(),
    };
    db.record_patch_build(build.clone())?;
    Ok(build)
}
