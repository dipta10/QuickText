use keyring::{Entry, Error};
use std::sync::Mutex;

#[derive(Default)]
pub struct CleanupCredentials(Mutex<Option<String>>);

fn entry() -> Result<Entry, String> {
    Entry::new("com.dipta.stt", "gemini-cleanup-api-key")
        .map_err(|_| "Could not open secure storage for the Gemini API key.".into())
}

impl CleanupCredentials {
    pub fn key(&self) -> Result<Option<String>, String> {
        let mut cached = self
            .0
            .lock()
            .map_err(|_| "Could not read cleanup credentials.")?;
        if let Some(key) = cached.as_ref() {
            return Ok(Some(key.clone()));
        }
        match entry()?.get_password() {
            Ok(key) if !key.trim().is_empty() => {
                *cached = Some(key.clone());
                Ok(Some(key))
            }
            Ok(_) | Err(Error::NoEntry) => Ok(None),
            Err(_) => Err("Could not read the Gemini API key from secure storage.".into()),
        }
    }

    pub fn save(&self, key: &str) -> Result<(), String> {
        let key = crate::sanitize_api_key(key);
        if key.is_empty() || key.chars().count() > crate::MAX_API_KEY_CHARACTERS {
            return Err("Enter a Gemini API key of 1–4,096 characters.".into());
        }
        let mut cached = self
            .0
            .lock()
            .map_err(|_| "Could not update cleanup credentials.")?;
        entry()?
            .set_password(&key)
            .map_err(|_| "Could not save the Gemini API key in secure storage.")?;
        *cached = Some(key);
        Ok(())
    }

    pub fn delete(&self) -> Result<(), String> {
        let mut cached = self
            .0
            .lock()
            .map_err(|_| "Could not update cleanup credentials.")?;
        match entry()?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => {
                *cached = None;
                Ok(())
            }
            Err(_) => Err("Could not delete the Gemini API key from secure storage.".into()),
        }
    }
}
