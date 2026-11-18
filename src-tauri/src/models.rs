use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub software: String,
    pub version: String,
    pub source_dir: Option<String>,
    pub exe_name: Option<String>,
    pub default_paths: Vec<String>,
    pub description: String,
    pub status: String, // draft | translating | reviewing | published
    pub total_units: i64,
    pub translated_units: i64,
    pub reviewed_units: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceFile {
    pub id: String,
    pub project_id: String,
    pub rel_path: String,
    pub target_rel_path: String,
    pub format: String, // json | yaml | properties | ini | po | xml | resx | ts | toml | arb
    pub original_sha256: String,
    pub unit_count: i64,
    pub translated_count: i64,
    pub raw_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationUnit {
    pub id: String,
    pub file_id: String,
    pub project_id: String,
    pub key_path: String,
    pub source: String,
    pub target: String,
    pub status: String, // untranslated | machine | translated | reviewed | flagged
    pub qa_warning: Option<String>,
    pub term_hits: Vec<String>,
    pub fuzzy_score: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntry {
    pub id: String,
    pub project_id: String, // "global" or project_id
    pub source: String,
    pub target: String,
    pub domain: String,
    pub context: String,
    pub do_not_translate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationMemory {
    pub id: String,
    pub source: String,
    pub target: String,
    pub domain: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProfile {
    pub id: String,
    pub name: String,
    pub provider: String, // deepseek | qwen | openai | ollama | custom
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub concurrency: i64,
    pub batch_size: i64,
    pub temperature: f64,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchBuild {
    pub id: String,
    pub project_id: String,
    pub version: String,
    pub build_type: String, // exe | zip
    pub output_path: String,
    pub file_count: i64,
    pub unit_count: i64,
    pub note: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedFileInfo {
    pub abs_path: String,
    pub rel_path: String,
    pub format: String,
    pub size_bytes: u64,
    pub preview_unit_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateProgressEvent {
    pub project_id: String,
    pub total: usize,
    pub completed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub message: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub project_count: i64,
    pub total_units: i64,
    pub translated_units: i64,
    pub reviewed_units: i64,
    pub qa_warning_units: i64,
    pub patch_count: i64,
    pub glossary_count: i64,
    pub memory_count: i64,
}
