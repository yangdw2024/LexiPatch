use crate::db::DbState;
use crate::llm;
use crate::models::{
    DashboardStats, GlossaryEntry, LlmProfile, PatchBuild, Project, ResourceFile, ScannedFileInfo,
    TranslationMemory, TranslationUnit,
};
use crate::packager;
use crate::parsers::{infer_format, patch_content};
use crate::scanner;
use serde::Deserialize;
use std::fs;
use tauri::{AppHandle, State};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileInput {
    pub abs_path: Option<String>,
    pub rel_path: String,
    pub target_rel_path: Option<String>,
    pub format: Option<String>,
    pub raw_content: Option<String>,
}

#[tauri::command]
pub fn get_dashboard_stats(db: State<DbState>) -> Result<DashboardStats, String> {
    db.get_dashboard_stats()
}

#[tauri::command]
pub fn list_projects(db: State<DbState>) -> Result<Vec<Project>, String> {
    db.list_projects()
}

#[tauri::command]
pub fn get_project(db: State<DbState>, id: String) -> Result<Option<Project>, String> {
    db.get_project(&id)
}

#[tauri::command]
pub fn create_project(
    db: State<DbState>,
    name: String,
    software: String,
    version: String,
    source_dir: Option<String>,
    exe_name: Option<String>,
    default_paths: Vec<String>,
    description: String,
) -> Result<Project, String> {
    db.create_project(name, software, version, source_dir, exe_name, default_paths, description)
}

#[tauri::command]
pub fn update_project_meta(
    db: State<DbState>,
    id: String,
    name: String,
    software: String,
    version: String,
    exe_name: Option<String>,
    default_paths: Vec<String>,
    description: String,
) -> Result<(), String> {
    db.update_project_meta(
        &id,
        &name,
        &software,
        &version,
        exe_name.as_deref(),
        &default_paths,
        &description,
    )
}

#[tauri::command]
pub fn delete_project(db: State<DbState>, id: String) -> Result<(), String> {
    db.delete_project(&id)
}

#[tauri::command]
pub fn scan_directory(dir_path: String) -> Result<Vec<ScannedFileInfo>, String> {
    scanner::scan_directory(&dir_path)
}

#[tauri::command]
pub fn import_project_files(
    db: State<DbState>,
    project_id: String,
    files: Vec<ImportFileInput>,
) -> Result<Vec<ResourceFile>, String> {
    let mut created = Vec::new();
    for f in files {
        let content = if let Some(raw) = f.raw_content {
            raw
        } else if let Some(abs) = f.abs_path {
            fs::read_to_string(&abs)
                .map_err(|e| format!("读取文件 {} 失败: {e}", abs))?
        } else {
            return Err(format!("文件 {} 缺少内容或路径", f.rel_path));
        };
        let fmt = f.format.unwrap_or_else(|| infer_format(&f.rel_path));
        let target = f.target_rel_path.unwrap_or_else(|| f.rel_path.clone());
        let rf = db.add_resource_file(&project_id, &f.rel_path, &target, &fmt, &content)?;
        created.push(rf);
    }
    Ok(created)
}

#[tauri::command]
pub fn list_files(db: State<DbState>, project_id: String) -> Result<Vec<ResourceFile>, String> {
    db.list_files(&project_id)
}

#[tauri::command]
pub fn update_file_target_path(
    db: State<DbState>,
    file_id: String,
    target_rel_path: String,
) -> Result<(), String> {
    db.update_file_target_path(&file_id, &target_rel_path)
}

#[tauri::command]
pub fn list_units(
    db: State<DbState>,
    project_id: String,
    file_id: Option<String>,
) -> Result<Vec<TranslationUnit>, String> {
    db.list_units(&project_id, file_id.as_deref())
}

#[tauri::command]
pub fn update_unit(
    db: State<DbState>,
    id: String,
    target: String,
    status: String,
) -> Result<Option<String>, String> {
    db.update_unit(&id, &target, &status)
}

#[tauri::command]
pub fn preview_patched_file(
    db: State<DbState>,
    project_id: String,
    file_id: String,
) -> Result<String, String> {
    let files = db.list_files(&project_id)?;
    let file = files
        .into_iter()
        .find(|f| f.id == file_id)
        .ok_or_else(|| "文件不存在".to_string())?;
    let units = db.list_units(&project_id, Some(&file_id))?;
    Ok(patch_content(&file.raw_content, &file.format, &units))
}

#[tauri::command]
pub fn list_glossary(
    db: State<DbState>,
    project_id: Option<String>,
) -> Result<Vec<GlossaryEntry>, String> {
    db.list_glossary(project_id.as_deref())
}

#[tauri::command]
pub fn upsert_glossary(db: State<DbState>, entry: GlossaryEntry) -> Result<(), String> {
    db.upsert_glossary(entry)
}

#[tauri::command]
pub fn delete_glossary(db: State<DbState>, id: String) -> Result<(), String> {
    db.delete_glossary(&id)
}

#[tauri::command]
pub fn list_memory(db: State<DbState>) -> Result<Vec<TranslationMemory>, String> {
    db.list_memory()
}

#[tauri::command]
pub fn list_llm_profiles(db: State<DbState>) -> Result<Vec<LlmProfile>, String> {
    db.list_llm_profiles()
}

#[tauri::command]
pub fn upsert_llm_profile(db: State<DbState>, profile: LlmProfile) -> Result<(), String> {
    db.upsert_llm_profile(profile)
}

#[tauri::command]
pub fn delete_llm_profile(db: State<DbState>, id: String) -> Result<(), String> {
    db.delete_llm_profile(&id)
}

#[tauri::command]
pub async fn test_llm_connection(profile: LlmProfile) -> Result<String, String> {
    llm::test_connection(&profile).await
}

#[tauri::command]
pub async fn run_llm_translate(
    app: AppHandle,
    db: State<'_, DbState>,
    project_id: String,
    file_id: Option<String>,
    profile_id: String,
    only_untranslated: bool,
) -> Result<(usize, usize, usize), String> {
    llm::run_project_translation(
        &app,
        &db,
        &project_id,
        file_id.as_deref(),
        &profile_id,
        only_untranslated,
    )
    .await
}

#[tauri::command]
pub fn build_exe_patcher(
    app: AppHandle,
    db: State<DbState>,
    project_id: String,
    version: String,
    author: String,
    note: String,
    output_path: String,
) -> Result<PatchBuild, String> {
    packager::build_exe_patcher(&app, &db, &project_id, &version, &author, &note, &output_path)
}

#[tauri::command]
pub fn build_zip_package(
    db: State<DbState>,
    project_id: String,
    version: String,
    note: String,
    output_path: String,
) -> Result<PatchBuild, String> {
    packager::build_zip_package(&db, &project_id, &version, &note, &output_path)
}

#[tauri::command]
pub fn list_patch_builds(
    db: State<DbState>,
    project_id: String,
) -> Result<Vec<PatchBuild>, String> {
    db.list_patch_builds(&project_id)
}
