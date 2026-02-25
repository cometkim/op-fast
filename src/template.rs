use std::collections::HashSet;

use fancy_regex::Regex;

lazy_static::lazy_static! {
    static ref UNENCLOSED_REF: Regex = Regex::new(
        r"(?<![A-Za-z0-9\-+\\.])(op://[A-Za-z0-9\-?_./\s]+)"
    ).unwrap();

    static ref ENCLOSED_REF: Regex = Regex::new(
        r"\{\{\s*(op://[^\}]+)\s*\}\}"
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
    subst::substitute(input, &subst::Env).unwrap_or_else(|_| input.to_string())
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
