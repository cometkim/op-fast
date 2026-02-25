use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use heed::{Database, Env, EnvOpenOptions};
use keyring_core::Entry as KeyringEntry;
use serde::{Deserialize, Serialize};

const DIR_NAME: &str = "op-fast";
const DB_NAME: &str = "store.mdb";
const SERVICE_NAME: &str = "op-fast";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub stored_at: u64,
    pub ttl_seconds: u64,
}

impl Meta {
    pub fn new(ttl: Duration) -> Self {
        let stored_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            stored_at,
            ttl_seconds: ttl.as_secs(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.stored_at + self.ttl_seconds
    }
}

pub struct Db {
    env: Env,
    db: Database<heed::types::Str, heed::types::SerdeBincode<Meta>>,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = Self::db_path()?;
        std::fs::create_dir_all(&path).context("Failed to create store directory")?;

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

    pub fn get(&self, reference: &str) -> Result<Option<String>> {
        let meta = {
            let rtxn = self.env.read_txn()?;
            let Some(meta) = self.db.get(&rtxn, reference)? else {
                return Ok(None);
            };
            meta
        };

        if meta.is_expired() {
            return Ok(None);
        }

        match Self::keyring_get(reference)? {
            Some(value) => Ok(Some(value)),
            None => {
                log::warn!(
                    "Store metadata exists but keyring value missing for: {}",
                    reference
                );
                Ok(None)
            }
        }
    }

    pub fn put(&self, reference: &str, value: &str, ttl: Duration) -> Result<()> {
        let meta = Meta::new(ttl);

        Self::keyring_put(reference, value)?;

        let mut wtxn = self.env.write_txn()?;
        if let Err(e) = self.db.put(&mut wtxn, reference, &meta) {
            let _ = Self::keyring_delete(reference);
            return Err(e.into());
        }
        wtxn.commit()?;

        Ok(())
    }

    pub fn delete(&self, reference: &str) -> Result<bool> {
        let deleted_keyring = Self::keyring_delete(reference)?;

        let mut wtxn = self.env.write_txn()?;
        let deleted_meta = self.db.delete(&mut wtxn, reference)?;
        wtxn.commit()?;

        Ok(deleted_keyring || deleted_meta)
    }

    pub fn list(&self) -> Result<Vec<(String, Meta)>> {
        let rtxn = self.env.read_txn()?;
        let iter = self.db.iter(&rtxn)?;
        let entries: Vec<_> = iter
            .filter_map(|r| r.ok())
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        Ok(entries)
    }

    pub fn clear(&self) -> Result<()> {
        let rtxn = self.env.read_txn()?;
        let iter = self.db.iter(&rtxn)?;
        for (reference, _) in iter.flatten() {
            let _ = Self::keyring_delete(reference);
        }
        drop(rtxn);

        let mut wtxn = self.env.write_txn()?;
        self.db.clear(&mut wtxn)?;
        wtxn.commit()?;

        Ok(())
    }

    pub fn gc(&self) -> Result<usize> {
        let entries = self.list()?;
        let mut deleted = 0;

        for (reference, meta) in &entries {
            if meta.is_expired() {
                self.delete(reference)?;
                deleted += 1;
            }
        }

        if deleted > 0 {
            log::info!("GC removed {} expired entries", deleted);
        }

        Ok(deleted)
    }

    fn db_path() -> Result<PathBuf> {
        if let Ok(path) = std::env::var("OP_FAST_STORE_DIR") {
            return Ok(PathBuf::from(path));
        }

        let data_dir = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs_sys::home_dir().map(|h| h.join(".local/share")))
            .context("Could not determine data directory")?;

        Ok(data_dir.join(DIR_NAME).join(DB_NAME))
    }

    fn keyring_entry(reference: &str) -> Result<KeyringEntry> {
        KeyringEntry::new(SERVICE_NAME, reference).context("Failed to create keyring entry")
    }

    fn keyring_get(reference: &str) -> Result<Option<String>> {
        let entry = Self::keyring_entry(reference)?;

        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Keyring error: {}", e)),
        }
    }

    fn keyring_put(reference: &str, value: &str) -> Result<()> {
        let entry = Self::keyring_entry(reference)?;
        entry
            .set_password(value)
            .context("Failed to store value in keyring")
    }

    fn keyring_delete(reference: &str) -> Result<bool> {
        let entry = Self::keyring_entry(reference)?;

        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring_core::Error::NoEntry) => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Keyring error: {}", e)),
        }
    }
}
