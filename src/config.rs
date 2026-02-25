use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use globset::Glob;
use serde::Deserialize;

const DEFAULT_TTL: Duration = Duration::from_secs(86400);

#[derive(Debug, Clone)]
pub struct Config {
    pub default_ttl: Duration,
    pub ttl_rules: Vec<TtlRule>,
}

#[derive(Debug, Clone)]
pub struct TtlRule {
    glob: Glob,
    ttl: Duration,
}

impl TtlRule {
    pub fn new(pattern: &str, ttl: Duration) -> Result<Self> {
        let glob = Glob::new(pattern)?;
        Ok(Self { glob, ttl })
    }

    pub fn matches(&self, reference: &str) -> bool {
        self.glob.compile_matcher().is_match(reference)
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        let raw = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("Failed to read config file")?;
            toml::from_str(&content)?
        } else {
            RawConfig::default()
        };

        let mut config = Self::from_raw(raw)?;

        if let Ok(env_ttl) = env::var("OP_FAST_DEFAULT_TTL") {
            config.default_ttl =
                humantime::parse_duration(&env_ttl).context("Invalid OP_FAST_DEFAULT_TTL")?;
        }

        Ok(config)
    }

    pub fn resolve_ttl(&self, reference: &str) -> Duration {
        for rule in &self.ttl_rules {
            if rule.matches(reference) {
                return rule.ttl;
            }
        }
        self.default_ttl
    }

    fn from_raw(raw: RawConfig) -> Result<Self> {
        let default_ttl = raw
            .default_ttl
            .map(|s| humantime::parse_duration(&s))
            .transpose()
            .context("Invalid default_ttl")?
            .unwrap_or(DEFAULT_TTL);

        let mut ttl_rules = Vec::new();
        for (pattern, ttl_str) in &raw.ttl {
            let ttl = humantime::parse_duration(ttl_str)
                .with_context(|| format!("Invalid TTL for pattern: {}", pattern))?;
            ttl_rules.push(TtlRule::new(pattern, ttl)?);
        }

        Ok(Self {
            default_ttl,
            ttl_rules,
        })
    }

    fn config_path() -> Result<PathBuf> {
        if let Ok(path) = env::var("OP_FAST_CONFIG") {
            return Ok(PathBuf::from(path));
        }

        let config_dir = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs_sys::home_dir().map(|h| h.join(".config")))
            .context("Could not determine config directory")?;

        Ok(config_dir.join("op-fast").join("config.toml"))
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    default_ttl: Option<String>,

    #[serde(default)]
    ttl: HashMap<String, String>,
}
