use crate::models::ScannedFileInfo;
use crate::parsers::{infer_format, is_supported_extension, parse_content};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".transcore_backup",
    "target",
    "__pycache__",
    ".venv",
    "dist",
    "build",
];

pub fn scan_directory(root_dir: &str) -> Result<Vec<ScannedFileInfo>, String> {
    let root = Path::new(root_dir);
    if !root.is_dir() {
        return Err("所选路径不是有效目录".into());
    }

    let mut results = Vec::new();
    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        if e.depth() > 0 && e.file_type().is_dir() {
            if let Some(name) = e.file_name().to_str() {
                return !SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(name));
            }
        }
        true
    });

    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_supported_extension(name) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        // Skip oversized files (> 8 MB)
        if meta.len() > 8 * 1024 * 1024 {
            continue;
        }
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let format = infer_format(name);
        let parsed = parse_content(&content, &format);
        if parsed.is_empty() {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        results.push(ScannedFileInfo {
            abs_path: path.to_string_lossy().to_string(),
            rel_path: rel,
            format,
            size_bytes: meta.len(),
            preview_unit_count: parsed.len(),
        });
    }

    results.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(results)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
