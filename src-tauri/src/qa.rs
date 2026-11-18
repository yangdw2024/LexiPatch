use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

static PLACEHOLDER_RE: OnceLock<Regex> = OnceLock::new();

fn placeholder_regex() -> &'static Regex {
    PLACEHOLDER_RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            (\{\{[^{}]+\}\})            # {{mustache}}
            |(\$\{[^{}]+\})             # ${template}
            |(\{[0-9a-zA-Z_.:-]+\})     # {0}, {name}
            |(%(?:\d+\$)?[+#0\- ]*(?:\d+)?(?:\.\d+)?[hlLzjt]*[diuoxXfFeEgGaAcspn%]) # printf %s, %1$d
            |(</?[a-zA-Z][a-zA-Z0-9-]*(?:\s+[^>]*)?/?>) # HTML/XML tags
            |(\\[ntr])                  # escape sequences \n \t \r
            "
        )
        .unwrap()
    })
}

pub fn extract_placeholders(text: &str) -> Vec<String> {
    placeholder_regex()
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Validates that all critical placeholders and escape sequences in `source` are preserved in `target`.
/// Returns `Some(warning_message)` if a problem is found, or `None` if clean.
pub fn validate_translation(source: &str, target: &str) -> Option<String> {
    let t = target.trim();
    if t.is_empty() {
        return None;
    }

    let src_ph = extract_placeholders(source);
    let tgt_ph = extract_placeholders(t);

    let mut src_counts: HashMap<&str, usize> = HashMap::new();
    for p in &src_ph {
        *src_counts.entry(p.as_str()).or_insert(0) += 1;
    }
    let mut tgt_counts: HashMap<&str, usize> = HashMap::new();
    for p in &tgt_ph {
        *tgt_counts.entry(p.as_str()).or_insert(0) += 1;
    }

    let mut missing = Vec::new();
    for (k, v) in &src_counts {
        let tv = tgt_counts.get(k).copied().unwrap_or(0);
        if tv < *v {
            missing.push(*k);
        }
    }
    if !missing.is_empty() {
        return Some(format!("缺少占位符或标签: {}", missing.join(", ")));
    }

    let mut extra = Vec::new();
    for (k, v) in &tgt_counts {
        let sv = src_counts.get(k).copied().unwrap_or(0);
        if *v > sv {
            extra.push(*k);
        }
    }
    if !extra.is_empty() {
        return Some(format!("多余占位符或标签: {}", extra.join(", ")));
    }

    None
}
