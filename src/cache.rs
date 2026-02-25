mod keyring;
mod store;

pub use keyring::KeyringStore;
pub use store::{CacheEntry, CacheStore};

use crate::config::Config;

pub fn init() -> anyhow::Result<()> {
    keyring::init()
}

pub struct Cache {
    store: CacheStore,
    config: Config,
}

impl Cache {
    pub fn open() -> anyhow::Result<Self> {
        let store = CacheStore::open()?;
        let config = Config::load()?;
        let cache = Self { store, config };

        // Run GC with 10% probability on open
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now.is_multiple_of(10)
            && let Err(e) = cache.gc() {
                log::warn!("Background GC failed: {}", e);
            }

        Ok(cache)
    }

    /// Garbage collect expired entries
    pub fn gc(&self) -> anyhow::Result<usize> {
        let entries = self.store.list()?;
        let mut deleted = 0;

        for (reference, entry) in &entries {
            if entry.is_expired() {
                self.store.delete(reference)?;
                let _ = KeyringStore::delete(reference);
                deleted += 1;
            }
        }

        if deleted > 0 {
            log::info!("GC removed {} expired entries", deleted);
        }

        Ok(deleted)
    }

    pub fn get(&self, reference: &str) -> anyhow::Result<Option<String>> {
        let _entry = match self.store.get(reference)? {
            Some(e) if !e.is_expired() => e,
            Some(_) => {
                log::debug!("Cache expired for: {}", reference);
                return Ok(None);
            }
            None => return Ok(None),
        };

        match KeyringStore::get(reference)? {
            Some(value) => {
                log::debug!("Cache hit for: {}", reference);
                Ok(Some(value))
            }
            None => {
                log::warn!(
                    "Cache metadata exists but keyring value missing for: {}",
                    reference
                );
                Ok(None)
            }
        }
    }

    pub fn put(&self, reference: &str, value: &str) -> anyhow::Result<()> {
        let ttl = self.config.resolve_ttl(reference);
        let entry = CacheEntry::new(ttl);

        self.store.put(reference, &entry)?;
        KeyringStore::put(reference, value)?;

        log::debug!("Cached {} with TTL {:?}", reference, ttl);
        Ok(())
    }

    pub fn delete(&self, reference: &str) -> anyhow::Result<bool> {
        let deleted_meta = self.store.delete(reference)?;
        let deleted_keyring = KeyringStore::delete(reference)?;
        Ok(deleted_meta || deleted_keyring)
    }

    pub fn list(&self) -> anyhow::Result<Vec<(String, CacheEntry)>> {
        self.store.list()
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        let entries = self.store.list()?;
        for (reference, _) in entries {
            let _ = KeyringStore::delete(&reference);
        }
        self.store.clear()?;
        Ok(())
    }
}
