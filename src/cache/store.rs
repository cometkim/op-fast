use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use heed::{Database, Env, EnvOpenOptions};
use serde::{Deserialize, Serialize};

const CACHE_DIR_NAME: &str = "op-offline";
const CACHE_DB_NAME: &str = "cache.mdb";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub cached_at: u64,
    pub ttl_seconds: u64,
}

impl CacheEntry {
    pub fn new(ttl: Duration) -> Self {
        let cached_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            cached_at,
            ttl_seconds: ttl.as_secs(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.cached_at + self.ttl_seconds
    }
}

pub struct CacheStore {
    env: Env,
    db: Database<heed::types::Str, heed::types::SerdeBincode<CacheEntry>>,
}

impl CacheStore {
    pub fn open() -> Result<Self> {
        let path = Self::cache_path()?;
        std::fs::create_dir_all(&path).context("Failed to create cache directory")?;

        let env = unsafe {
            EnvOpenOptions::new()
                .max_dbs(1)
                .map_size(10 * 1024 * 1024)
                .open(&path)?
        };

        let mut wtxn = env.write_txn()?;
        let db = env.create_database(&mut wtxn, None)?;
        wtxn.commit()?;

        Ok(Self { env, db })
    }

    pub fn get(&self, reference: &str) -> Result<Option<CacheEntry>> {
        let rtxn = self.env.read_txn()?;
        let entry = self.db.get(&rtxn, reference)?;
        Ok(entry)
    }

    pub fn put(&self, reference: &str, entry: &CacheEntry) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.db.put(&mut wtxn, reference, entry)?;
        wtxn.commit()?;
        Ok(())
    }

    pub fn delete(&self, reference: &str) -> Result<bool> {
        let mut wtxn = self.env.write_txn()?;
        let deleted = self.db.delete(&mut wtxn, reference)?;
        wtxn.commit()?;
        Ok(deleted)
    }

    pub fn list(&self) -> Result<Vec<(String, CacheEntry)>> {
        let rtxn = self.env.read_txn()?;
        let iter = self.db.iter(&rtxn)?;
        let entries: Vec<_> = iter
            .filter_map(|r| r.ok())
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        Ok(entries)
    }

    pub fn clear(&self) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.db.clear(&mut wtxn)?;
        wtxn.commit()?;
        Ok(())
    }

    fn cache_path() -> Result<PathBuf> {
        if let Ok(path) = std::env::var("OP_OFFLINE_CACHE_DIR") {
            return Ok(PathBuf::from(path));
        }

        let data_dir = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs_sys::home_dir().map(|h| h.join(".local/share")))
            .context("Could not determine data directory")?;

        Ok(data_dir.join(CACHE_DIR_NAME).join(CACHE_DB_NAME))
    }
}
