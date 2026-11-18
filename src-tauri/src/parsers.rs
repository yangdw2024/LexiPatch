use crate::models::TranslationUnit;
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer};
use std::collections::HashMap;
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct ParsedEntry {
    pub key_path: String,
    pub source: String,
    pub existing_target: String,
}

pub fn infer_format(filename: &str) -> String {
    let lower = filename.to_ascii_lowercase();
    if lower.ends_with(".json") {
        "json".into()
    } else if lower.ends_with(".arb") {
        "arb".into()
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        "yaml".into()
    } else if lower.ends_with(".toml") {
        "toml".into()
    } else if lower.ends_with(".properties") {
        "properties".into()
    } else if lower.ends_with(".ini") || lower.ends_with(".cfg") || lower.ends_with(".conf") {
        "ini".into()
    } else if lower.ends_with(".po") || lower.ends_with(".pot") {
        "po".into()
    } else if lower.ends_with(".resx") {
        "resx".into()
    } else if lower.ends_with(".ts") {
        "ts".into()
    } else if lower.ends_with(".xml") {
        "xml".into()
    } else {
        "txt".into()
    }
}

pub fn is_supported_extension(filename: &str) -> bool {
    let fmt = infer_format(filename);
    fmt != "txt"
}

pub fn parse_content(content: &str, format: &str) -> Vec<ParsedEntry> {
    match format {
        "json" | "arb" => parse_json(content, format == "arb"),
        "yaml" => parse_yaml(content),
        "toml" => parse_toml(content),
        "properties" => parse_properties(content),
        "ini" => parse_ini(content),
        "po" => parse_po(content),
        "xml" | "resx" | "ts" => parse_xml_like(content),
        _ => Vec::new(),
    }
}

/// Format-preserving patch: applies translated `units` onto `raw_content`.
pub fn patch_content(raw_content: &str, format: &str, units: &[TranslationUnit]) -> String {
    let map: HashMap<&str, &str> = units
        .iter()
        .map(|u| {
            let val = if u.target.trim().is_empty() {
                u.source.as_str()
            } else {
                u.target.as_str()
            };
            (u.key_path.as_str(), val)
        })
        .collect();

    match format {
        "json" | "arb" => patch_json(raw_content, &map),
        "yaml" => patch_yaml(raw_content, &map),
        "toml" => patch_toml(raw_content, &map),
        "properties" => patch_properties(raw_content, &map),
        "ini" => patch_ini(raw_content, &map),
        "po" => patch_po(raw_content, &map),
        "xml" | "resx" | "ts" => patch_xml_like(raw_content, &map),
        _ => raw_content.to_string(),
    }
}

// ==================== JSON / ARB ====================

fn parse_json(content: &str, is_arb: bool) -> Vec<ParsedEntry> {
    let mut out = Vec::new();
    let Ok(val) = serde_json::from_str::<serde_json::Value>(content) else {
        return out;
    };
    fn walk(node: &serde_json::Value, prefix: &str, is_arb: bool, out: &mut Vec<ParsedEntry>) {
        match node {
            serde_json::Value::String(s) => {
                if !prefix.is_empty() && !s.trim().is_empty() {
                    if is_arb && (prefix.starts_with('@') || prefix == "@@locale") {
                        return;
                    }
                    out.push(ParsedEntry {
                        key_path: prefix.to_string(),
                        source: s.clone(),
                        existing_target: String::new(),
                    });
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    let next = format!("{}[{}]", prefix, i);
                    walk(item, &next, is_arb, out);
                }
            }
            serde_json::Value::Object(obj) => {
                for (k, v) in obj {
                    if is_arb && k.starts_with('@') {
                        continue;
                    }
                    let next = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    walk(v, &next, is_arb, out);
                }
            }
            _ => {}
        }
    }
    walk(&val, "", is_arb, &mut out);
    out
}

fn patch_json(raw: &str, map: &HashMap<&str, &str>) -> String {
    let Ok(mut root) = serde_json::from_str::<serde_json::Value>(raw) else {
        return raw.to_string();
    };
    fn walk_mut(node: &mut serde_json::Value, prefix: &str, map: &HashMap<&str, &str>) {
        match node {
            serde_json::Value::String(s) => {
                if let Some(new_val) = map.get(prefix) {
                    *s = (*new_val).to_string();
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, item) in arr.iter_mut().enumerate() {
                    let next = format!("{}[{}]", prefix, i);
                    walk_mut(item, &next, map);
                }
            }
            serde_json::Value::Object(obj) => {
                for (k, v) in obj.iter_mut() {
                    let next = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    walk_mut(v, &next, map);
                }
            }
            _ => {}
        }
    }
    walk_mut(&mut root, "", map);
    serde_json::to_string_pretty(&root).unwrap_or_else(|_| raw.to_string()) + "\n"
}

// ==================== YAML ====================

fn parse_yaml(content: &str) -> Vec<ParsedEntry> {
    let mut out = Vec::new();
    let Ok(val) = serde_yml::from_str::<serde_yml::Value>(content) else {
        return out;
    };
    fn walk(node: &serde_yml::Value, prefix: &str, out: &mut Vec<ParsedEntry>) {
        match node {
            serde_yml::Value::String(s) => {
                if !prefix.is_empty() && !s.trim().is_empty() {
                    out.push(ParsedEntry {
                        key_path: prefix.to_string(),
                        source: s.clone(),
                        existing_target: String::new(),
                    });
                }
            }
            serde_yml::Value::Sequence(seq) => {
                for (i, item) in seq.iter().enumerate() {
                    let next = format!("{}[{}]", prefix, i);
                    walk(item, &next, out);
                }
            }
            serde_yml::Value::Mapping(m) => {
                for (k, v) in m {
                    if let Some(ks) = k.as_str() {
                        let next = if prefix.is_empty() {
                            ks.to_string()
                        } else {
                            format!("{}.{}", prefix, ks)
                        };
                        walk(v, &next, out);
                    }
                }
            }
            _ => {}
        }
    }
    walk(&val, "", &mut out);
    out
}

fn patch_yaml(raw: &str, map: &HashMap<&str, &str>) -> String {
    let Ok(mut root) = serde_yml::from_str::<serde_yml::Value>(raw) else {
        return raw.to_string();
    };
    fn walk_mut(node: &mut serde_yml::Value, prefix: &str, map: &HashMap<&str, &str>) {
        match node {
            serde_yml::Value::String(s) => {
                if let Some(new_val) = map.get(prefix) {
                    *s = (*new_val).to_string();
                }
            }
            serde_yml::Value::Sequence(seq) => {
                for (i, item) in seq.iter_mut().enumerate() {
                    let next = format!("{}[{}]", prefix, i);
                    walk_mut(item, &next, map);
                }
            }
            serde_yml::Value::Mapping(m) => {
                for (k, v) in m.iter_mut() {
                    if let Some(ks) = k.as_str() {
                        let next = if prefix.is_empty() {
                            ks.to_string()
                        } else {
                            format!("{}.{}", prefix, ks)
                        };
                        walk_mut(v, &next, map);
                    }
                }
            }
            _ => {}
        }
    }
    walk_mut(&mut root, "", map);
    serde_yml::to_string(&root).unwrap_or_else(|_| raw.to_string())
}

// ==================== TOML (100% comment/format preserving) ====================

fn parse_toml(content: &str) -> Vec<ParsedEntry> {
    let mut out = Vec::new();
    let Ok(doc) = content.parse::<toml_edit::DocumentMut>() else {
        return out;
    };
    fn walk_table(table: &toml_edit::Table, prefix: &str, out: &mut Vec<ParsedEntry>) {
        for (k, item) in table.iter() {
            let next = if prefix.is_empty() {
                k.to_string()
            } else {
                format!("{}.{}", prefix, k)
            };
            match item {
                toml_edit::Item::Value(toml_edit::Value::String(s)) => {
                    if !s.value().trim().is_empty() {
                        out.push(ParsedEntry {
                            key_path: next,
                            source: s.value().clone(),
                            existing_target: String::new(),
                        });
                    }
                }
                toml_edit::Item::Table(sub) => walk_table(sub, &next, out),
                _ => {}
            }
        }
    }
    walk_table(doc.as_table(), "", &mut out);
    out
}

fn patch_toml(raw: &str, map: &HashMap<&str, &str>) -> String {
    let Ok(mut doc) = raw.parse::<toml_edit::DocumentMut>() else {
        return raw.to_string();
    };
    fn walk_mut(table: &mut toml_edit::Table, prefix: &str, map: &HashMap<&str, &str>) {
        for (k, item) in table.iter_mut() {
            let next = if prefix.is_empty() {
                k.to_string()
            } else {
                format!("{}.{}", prefix, k)
            };
            match item {
                toml_edit::Item::Value(v) => {
                    if v.is_str() {
                        if let Some(new_val) = map.get(next.as_str()) {
                            let decor = v.decor().clone();
                            let mut new_v = toml_edit::Value::from(*new_val);
                            *new_v.decor_mut() = decor;
                            *v = new_v;
                        }
                    }
                }
                toml_edit::Item::Table(sub) => walk_mut(sub, &next, map),
                _ => {}
            }
        }
    }
    walk_mut(doc.as_table_mut(), "", map);
    doc.to_string()
}

// ==================== Properties (comment/format preserving) ====================

fn parse_properties(content: &str) -> Vec<ParsedEntry> {
    let mut out = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            continue;
        }
        if let Some((k, v)) = split_prop_line(line) {
            let key = k.trim();
            let val = v.trim();
            if !key.is_empty() && !val.is_empty() {
                out.push(ParsedEntry {
                    key_path: key.to_string(),
                    source: val.to_string(),
                    existing_target: String::new(),
                });
            }
        }
    }
    out
}

fn split_prop_line(line: &str) -> Option<(&str, &str)> {
    let eq = line.find('=');
    let colon = line.find(':');
    let sep = match (eq, colon) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }?;
    Some((&line[..sep], &line[sep + 1..]))
}

fn patch_properties(raw: &str, map: &HashMap<&str, &str>) -> String {
    let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            lines.push(line.to_string());
            continue;
        }
        if let Some((left, right)) = split_prop_line(line) {
            let key = left.trim();
            if let Some(new_val) = map.get(key) {
                let sep_char = line.chars().nth(left.len()).unwrap_or('=');
                let leading_ws: String = right.chars().take_while(|c| c.is_whitespace()).collect();
                lines.push(format!("{}{}{}{}", left, sep_char, leading_ws, new_val));
                continue;
            }
        }
        lines.push(line.to_string());
    }
    lines.join(eol) + eol
}

// ==================== INI / CFG / CONF ====================

fn parse_ini(content: &str) -> Vec<ParsedEntry> {
    let mut out = Vec::new();
    let mut section = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
            section = trimmed[1..trimmed.len() - 1].trim().to_string();
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let val = line[eq + 1..].trim();
            if !key.is_empty() && !val.is_empty() {
                let full_key = if section.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", section, key)
                };
                out.push(ParsedEntry {
                    key_path: full_key,
                    source: val.to_string(),
                    existing_target: String::new(),
                });
            }
        }
    }
    out
}

fn patch_ini(raw: &str, map: &HashMap<&str, &str>) -> String {
    let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
    let mut section = String::new();
    let mut out_lines = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            out_lines.push(line.to_string());
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() >= 2 {
            section = trimmed[1..trimmed.len() - 1].trim().to_string();
            out_lines.push(line.to_string());
            continue;
        }
        if let Some(eq) = line.find('=') {
            let left = &line[..eq];
            let right = &line[eq + 1..];
            let key = left.trim();
            let full_key = if section.is_empty() {
                key.to_string()
            } else {
                format!("{}.{}", section, key)
            };
            if let Some(new_val) = map.get(full_key.as_str()) {
                let leading_ws: String = right.chars().take_while(|c| c.is_whitespace()).collect();
                out_lines.push(format!("{}={}{}", left, leading_ws, new_val));
                continue;
            }
        }
        out_lines.push(line.to_string());
    }
    out_lines.join(eol) + eol
}

// ==================== Gettext PO / POT ====================

fn unquote_po(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\\\", "\\")
    } else {
        t.to_string()
    }
}

fn escape_po(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn parse_po(content: &str) -> Vec<ParsedEntry> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0usize;
    let mut idx = 0usize;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("msgid ") {
            let mut msgid = unquote_po(line.trim_start_matches("msgid "));
            i += 1;
            while i < lines.len() && lines[i].trim().starts_with('"') {
                msgid.push_str(&unquote_po(lines[i]));
                i += 1;
            }
            let mut msgstr = String::new();
            if i < lines.len() && lines[i].trim().starts_with("msgstr ") {
                msgstr = unquote_po(lines[i].trim().trim_start_matches("msgstr "));
                i += 1;
                while i < lines.len() && lines[i].trim().starts_with('"') {
                    msgstr.push_str(&unquote_po(lines[i]));
                    i += 1;
                }
            }
            if !msgid.is_empty() {
                out.push(ParsedEntry {
                    key_path: format!("msg_{}", idx),
                    source: msgid,
                    existing_target: msgstr,
                });
                idx += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn patch_po(raw: &str, map: &HashMap<&str, &str>) -> String {
    let eol = if raw.contains("\r\n") { "\r\n" } else { "\n" };
    let lines: Vec<&str> = raw.lines().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut idx = 0usize;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.starts_with("msgid ") {
            let mut msgid = unquote_po(trimmed.trim_start_matches("msgid "));
            out.push(line.to_string());
            i += 1;
            while i < lines.len() && lines[i].trim().starts_with('"') {
                msgid.push_str(&unquote_po(lines[i]));
                out.push(lines[i].to_string());
                i += 1;
            }
            if !msgid.is_empty() {
                let key = format!("msg_{}", idx);
                idx += 1;
                if i < lines.len() && lines[i].trim().starts_with("msgstr ") {
                    if let Some(new_val) = map.get(key.as_str()) {
                        out.push(format!("msgstr \"{}\"", escape_po(new_val)));
                        i += 1;
                        while i < lines.len() && lines[i].trim().starts_with('"') {
                            i += 1;
                        }
                        continue;
                    }
                }
            }
        } else {
            out.push(line.to_string());
            i += 1;
        }
    }
    out.join(eol) + eol
}

// ==================== XML / RESX / Qt .ts ====================

fn parse_xml_like(content: &str) -> Vec<ParsedEntry> {
    let mut out = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);

    let mut current_data_name: Option<String> = None;
    let mut in_value = false;
    let mut current_string_name: Option<String> = None;
    let mut in_ts_message = false;
    let mut in_ts_source = false;
    let mut in_ts_translation = false;
    let mut ts_source = String::new();
    let mut ts_trans = String::new();
    let mut ts_idx = 0usize;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "data" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                current_data_name =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    "value" if current_data_name.is_some() => in_value = true,
                    "string" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                current_string_name =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    "message" => {
                        in_ts_message = true;
                        ts_source.clear();
                        ts_trans.clear();
                    }
                    "source" if in_ts_message => in_ts_source = true,
                    "translation" if in_ts_message => in_ts_translation = true,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let txt = e.unescape().map(|s| s.to_string()).unwrap_or_default();
                if in_value {
                    if let Some(ref key) = current_data_name {
                        if !txt.trim().is_empty() {
                            out.push(ParsedEntry {
                                key_path: key.clone(),
                                source: txt,
                                existing_target: String::new(),
                            });
                        }
                    }
                } else if let Some(ref key) = current_string_name {
                    if !txt.trim().is_empty() {
                        out.push(ParsedEntry {
                            key_path: key.clone(),
                            source: txt,
                            existing_target: String::new(),
                        });
                    }
                } else if in_ts_source {
                    ts_source.push_str(&txt);
                } else if in_ts_translation {
                    ts_trans.push_str(&txt);
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "data" => current_data_name = None,
                    "value" => in_value = false,
                    "string" => current_string_name = None,
                    "source" => in_ts_source = false,
                    "translation" => in_ts_translation = false,
                    "message" => {
                        if in_ts_message && !ts_source.trim().is_empty() {
                            out.push(ParsedEntry {
                                key_path: format!("ts_{}", ts_idx),
                                source: ts_source.clone(),
                                existing_target: ts_trans.clone(),
                            });
                            ts_idx += 1;
                        }
                        in_ts_message = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out
}

fn patch_xml_like(raw: &str, map: &HashMap<&str, &str>) -> String {
    let mut reader = Reader::from_str(raw);
    reader.config_mut().trim_text(false);
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    let mut current_data_name: Option<String> = None;
    let mut in_value = false;
    let mut current_string_name: Option<String> = None;
    let mut in_ts_message = false;
    let mut in_ts_translation = false;
    let mut ts_idx = 0usize;
    let mut replaced_current_node = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut elem_to_write = e.clone();
                match name.as_str() {
                    "data" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                current_data_name =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    "value" if current_data_name.is_some() => {
                        in_value = true;
                        replaced_current_node = false;
                    }
                    "string" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                current_string_name =
                                    Some(String::from_utf8_lossy(&attr.value).to_string());
                                replaced_current_node = false;
                            }
                        }
                    }
                    "message" => in_ts_message = true,
                    "translation" if in_ts_message => {
                        in_ts_translation = true;
                        replaced_current_node = false;
                        // Remove type="unfinished" if we have a translation
                        let key = format!("ts_{}", ts_idx);
                        if map.contains_key(key.as_str()) {
                            let mut cleaned = quick_xml::events::BytesStart::new("translation");
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() != b"type" {
                                    cleaned.push_attribute(attr);
                                }
                            }
                            elem_to_write = cleaned;
                        }
                    }
                    _ => {}
                }
                let _ = writer.write_event(Event::Start(elem_to_write));
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == "translation" && in_ts_message {
                    let key = format!("ts_{}", ts_idx);
                    if let Some(new_val) = map.get(key.as_str()) {
                        let start = quick_xml::events::BytesStart::new("translation");
                        let end = quick_xml::events::BytesEnd::new("translation");
                        let _ = writer.write_event(Event::Start(start));
                        let _ = writer.write_event(Event::Text(BytesText::new(new_val)));
                        let _ = writer.write_event(Event::End(end));
                        continue;
                    }
                }
                let _ = writer.write_event(Event::Empty(e));
            }
            Ok(Event::Text(e)) => {
                if in_value {
                    if let Some(ref key) = current_data_name {
                        if let Some(new_val) = map.get(key.as_str()) {
                            if !replaced_current_node {
                                let _ = writer.write_event(Event::Text(BytesText::new(new_val)));
                                replaced_current_node = true;
                            }
                            continue;
                        }
                    }
                } else if let Some(ref key) = current_string_name {
                    if let Some(new_val) = map.get(key.as_str()) {
                        if !replaced_current_node {
                            let _ = writer.write_event(Event::Text(BytesText::new(new_val)));
                            replaced_current_node = true;
                        }
                        continue;
                    }
                } else if in_ts_translation {
                    let key = format!("ts_{}", ts_idx);
                    if let Some(new_val) = map.get(key.as_str()) {
                        if !replaced_current_node {
                            let _ = writer.write_event(Event::Text(BytesText::new(new_val)));
                            replaced_current_node = true;
                        }
                        continue;
                    }
                }
                let _ = writer.write_event(Event::Text(e));
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "data" => current_data_name = None,
                    "value" => in_value = false,
                    "string" => current_string_name = None,
                    "translation" => {
                        if in_ts_translation && !replaced_current_node {
                            let key = format!("ts_{}", ts_idx);
                            if let Some(new_val) = map.get(key.as_str()) {
                                let _ = writer.write_event(Event::Text(BytesText::new(new_val)));
                            }
                        }
                        in_ts_translation = false;
                    }
                    "message" => {
                        if in_ts_message {
                            ts_idx += 1;
                        }
                        in_ts_message = false;
                    }
                    _ => {}
                }
                let _ = writer.write_event(Event::End(e));
            }
            Ok(Event::Eof) => break,
            Ok(ev) => {
                let _ = writer.write_event(ev);
            }
            Err(_) => return raw.to_string(),
        }
    }

    String::from_utf8(writer.into_inner().into_inner()).unwrap_or_else(|_| raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ini_preserves_comments() {
        let raw = "; Header comment\n[Menu]\nFile = File\n; keep me\nPort = 8080\n";
        let entries = parse_content(raw, "ini");
        assert_eq!(entries.len(), 2);
        let units = vec![TranslationUnit {
            id: "1".into(),
            file_id: "f".into(),
            project_id: "p".into(),
            key_path: "Menu.File".into(),
            source: "File".into(),
            target: "文件".into(),
            status: "translated".into(),
            qa_warning: None,
            term_hits: vec![],
            fuzzy_score: 100,
            updated_at: 0,
        }];
        let patched = patch_content(raw, "ini", &units);
        assert!(patched.contains("; Header comment"));
        assert!(patched.contains("; keep me"));
        assert!(patched.contains("File = 文件"));
        assert!(patched.contains("Port = 8080"));
    }

    #[test]
    fn test_toml_preserves_comments() {
        let raw = "# App config\n[ui]\ntitle = \"My App\" # inline\ncount = 5\n";
        let entries = parse_content(raw, "toml");
        assert_eq!(entries.len(), 1);
        let units = vec![TranslationUnit {
            id: "1".into(),
            file_id: "f".into(),
            project_id: "p".into(),
            key_path: "ui.title".into(),
            source: "My App".into(),
            target: "我的应用".into(),
            status: "translated".into(),
            qa_warning: None,
            term_hits: vec![],
            fuzzy_score: 100,
            updated_at: 0,
        }];
        let patched = patch_content(raw, "toml", &units);
        assert!(patched.contains("# App config"));
        assert!(patched.contains("title = \"我的应用\" # inline"));
        assert!(patched.contains("count = 5"));
    }

    #[test]
    fn test_qt_ts_patches_all_forms() {
        let raw = r#"<?xml version="1.0" encoding="utf-8"?>
<TS version="2.1" language="zh_CN">
<context><name>QObject</name>
  <message><source>Both</source><translation>Both</translation></message>
  <message><source>Save</source><translation type="unfinished"></translation></message>
  <message><source>Quit</source><translation type="unfinished"/></message>
</context>
</TS>"#;
        let entries = parse_content(raw, "ts");
        assert_eq!(entries.len(), 3);
        let units = vec![
            TranslationUnit {
                id: "1".into(), file_id: "f".into(), project_id: "p".into(),
                key_path: "ts_0".into(), source: "Both".into(), target: "双向".into(),
                status: "translated".into(), qa_warning: None, term_hits: vec![], fuzzy_score: 100, updated_at: 0,
            },
            TranslationUnit {
                id: "2".into(), file_id: "f".into(), project_id: "p".into(),
                key_path: "ts_1".into(), source: "Save".into(), target: "保存".into(),
                status: "translated".into(), qa_warning: None, term_hits: vec![], fuzzy_score: 100, updated_at: 0,
            },
            TranslationUnit {
                id: "3".into(), file_id: "f".into(), project_id: "p".into(),
                key_path: "ts_2".into(), source: "Quit".into(), target: "退出".into(),
                status: "translated".into(), qa_warning: None, term_hits: vec![], fuzzy_score: 100, updated_at: 0,
            },
        ];
        let patched = patch_content(raw, "ts", &units);
        assert!(patched.contains("<translation>双向</translation>"));
        assert!(patched.contains("<translation>保存</translation>"));
        assert!(patched.contains("<translation>退出</translation>"));
        assert!(!patched.contains("unfinished"));
    }
}
