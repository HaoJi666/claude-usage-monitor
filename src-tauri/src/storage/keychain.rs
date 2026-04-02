use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "claude-usage-monitor";

pub fn save_credential(account: &str, token: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, account)
        .context("Failed to create keychain entry")?;
    entry.set_password(token)
        .context("Failed to save to keychain")?;
    Ok(())
}

pub fn get_credential(account: &str) -> Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, account)
        .context("Failed to create keychain entry")?;
    match entry.get_password() {
        Ok(password) => Ok(Some(password)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Keychain error: {}", e)),
    }
}

pub fn delete_credential(account: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, account)
        .context("Failed to create keychain entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(anyhow::anyhow!("Failed to delete from keychain: {}", e)),
    }
}
