mod commands;
mod db;
mod llm;
mod models;
mod packager;
mod parsers;
mod qa;
mod scanner;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
            let db_path = app_dir.join("lexipatch.db");
            let state = db::init_db(&db_path)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_dashboard_stats,
            commands::list_projects,
            commands::get_project,
            commands::create_project,
            commands::update_project_meta,
            commands::delete_project,
            commands::scan_directory,
            commands::import_project_files,
            commands::list_files,
            commands::update_file_target_path,
            commands::list_units,
            commands::update_unit,
            commands::preview_patched_file,
            commands::list_glossary,
            commands::upsert_glossary,
            commands::delete_glossary,
            commands::list_memory,
            commands::list_llm_profiles,
            commands::upsert_llm_profile,
            commands::delete_llm_profile,
            commands::fetch_llm_models,
            commands::test_llm_connection,
            commands::run_llm_translate,
            commands::build_exe_patcher,
            commands::build_zip_package,
            commands::list_patch_builds,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
