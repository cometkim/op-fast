use anyhow::{Context, Result};
use keyring_core::Entry;

const SERVICE_NAME: &str = "op-offline";

pub struct KeyringStore;

impl KeyringStore {
    pub fn get(reference: &str) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, reference)?;

        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Keyring error: {}", e)),
        }
    }

    pub fn put(reference: &str, value: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, reference)?;
        entry
            .set_password(value)
            .context("Failed to store value in keyring")
    }

    pub fn delete(reference: &str) -> Result<bool> {
        let entry = Entry::new(SERVICE_NAME, reference)?;

        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring_core::Error::NoEntry) => Ok(false),
            Err(e) => Err(anyhow::anyhow!("Keyring error: {}", e)),
        }
    }
}

pub fn init() -> Result<()> {
    keyring::use_native_store(false).context("Failed to initialize native keyring store")
}
