use std::collections::HashSet;

use fancy_regex::Regex;

lazy_static::lazy_static! {
    static ref UNENCLOSED_REF: Regex = Regex::new(
        r"(?<![A-Za-z0-9\-+\\.])(op://[A-Za-z0-9\-?_./\s]+)"
    ).unwrap();

    static ref ENCLOSED_REF: Regex = Regex::new(
        r"\{\{\s*(op://[^\}]+)\s*\}\}"
    ).unwrap();

    static ref UNENCLOSED_VAR: Regex = Regex::new(
        r"\$([A-Za-z_][A-Za-z0-9_]*)"
    ).unwrap();

    static ref VAR_WITH_DEFAULT: Regex = Regex::new(
        r"\$\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*:-(.*?)\}"
    ).unwrap();

    static ref ENCLOSED_VAR: Regex = Regex::new(
        r"\$\{\s*([A-Za-z_][A-Za-z0-9_]*)\s*\}"
    ).unwrap();
}

pub fn extract_references(input: &str) -> HashSet<String> {
    let mut refs = HashSet::new();

    for cap in UNENCLOSED_REF.captures_iter(input).flatten() {
        if let Some(m) = cap.get(1) {
            refs.insert(m.as_str().trim().to_string());
        }
    }

    for cap in ENCLOSED_REF.captures_iter(input).flatten() {
        if let Some(m) = cap.get(1) {
            refs.insert(m.as_str().trim().to_string());
        }
    }

    refs
}

pub fn resolve_variables(input: &str) -> String {
    let input = VAR_WITH_DEFAULT.replace_all(input, |caps: &fancy_regex::Captures| {
        let var_name = caps.get(1).unwrap().as_str();
        let default = caps.get(2).unwrap().as_str();
        std::env::var(var_name).unwrap_or_else(|_| default.to_string())
    });

    let input = ENCLOSED_VAR.replace_all(&input, |caps: &fancy_regex::Captures| {
        let var_name = caps.get(1).unwrap().as_str().trim();
        std::env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name))
    });

    UNENCLOSED_VAR
        .replace_all(&input, |caps: &fancy_regex::Captures| {
            let var_name = caps.get(1).unwrap().as_str();
            std::env::var(var_name).unwrap_or_else(|_| format!("${}", var_name))
        })
        .into_owned()
}

pub fn substitute_references<F>(input: &str, resolver: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let input = ENCLOSED_REF.replace_all(input, |caps: &fancy_regex::Captures| {
        let reference = caps.get(1).unwrap().as_str().trim();
        resolver(reference).unwrap_or_else(|| caps.get(0).unwrap().as_str().to_string())
    });

    UNENCLOSED_REF
        .replace_all(&input, |caps: &fancy_regex::Captures| {
            let reference = caps.get(1).unwrap().as_str().trim();
            resolver(reference).unwrap_or_else(|| caps.get(0).unwrap().as_str().to_string())
        })
        .into_owned()
}
