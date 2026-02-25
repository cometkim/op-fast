mod db;

pub use db::{Db, Meta};

use crate::config::Config;

pub fn init() -> anyhow::Result<()> {
    keyring::use_native_store(false).map_err(|e| anyhow::anyhow!("Failed to initialize native keyring store: {}", e))
}

pub struct Store {
    db: Db,
    config: Config,
}

impl Store {
    pub fn open() -> anyhow::Result<Self> {
        let db = Db::open()?;
        let config = Config::load()?;
        let store = Self { db, config };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now.is_multiple_of(10)
            && let Err(e) = store.db.gc() {
                log::warn!("Background GC failed: {}", e);
            }

        Ok(store)
    }

    pub fn get(&self, reference: &str) -> anyhow::Result<Option<String>> {
        match self.db.get(reference)? {
            Some(value) => {
                log::debug!("Store hit for: {}", reference);
                Ok(Some(value))
            }
            None => {
                log::debug!("Store miss for: {}", reference);
                Ok(None)
            }
        }
    }

    pub fn put(&self, reference: &str, value: &str) -> anyhow::Result<()> {
        let ttl = self.config.resolve_ttl(reference);
        self.db.put(reference, value, ttl)?;
        log::debug!("Stored {} with TTL {:?}", reference, ttl);
        Ok(())
    }

    pub fn delete(&self, reference: &str) -> anyhow::Result<bool> {
        self.db.delete(reference)
    }

    pub fn list(&self) -> anyhow::Result<Vec<(String, Meta)>> {
        self.db.list()
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        self.db.clear()
    }
}
