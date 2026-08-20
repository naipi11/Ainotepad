use std::path::PathBuf;

use crate::config::config_dir;

#[derive(Debug)]
pub enum SecretError {
    Io(String),
    Protect(String),
    InvalidProfileId,
}

pub trait SecretStore {
    fn write(&self, bytes: &[u8]) -> Result<(), SecretError>;
    fn read(&self) -> Result<Option<Vec<u8>>, SecretError>;
}

pub struct FileSecretStore {
    path: PathBuf,
}

impl FileSecretStore {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_store() -> Self {
        Self::at(config_dir().join("api_key.dpapi"))
    }
}

impl SecretStore for FileSecretStore {
    fn write(&self, bytes: &[u8]) -> Result<(), SecretError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SecretError::Io(e.to_string()))?;
        }
        if bytes.is_empty() {
            let _ = std::fs::remove_file(&self.path);
            return Ok(());
        }
        let protected = protect(bytes)?;
        std::fs::write(&self.path, protected).map_err(|e| SecretError::Io(e.to_string()))
    }

    fn read(&self) -> Result<Option<Vec<u8>>, SecretError> {
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(Some(unprotect(&bytes)?)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(SecretError::Io(err.to_string())),
        }
    }
}

pub fn store_api_key(key: &str) -> Result<(), SecretError> {
    FileSecretStore::default_store().write(key.as_bytes())
}

pub fn load_api_key() -> Result<Option<String>, SecretError> {
    Ok(FileSecretStore::default_store()
        .read()?
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}

pub fn profile_secret_path(profile_id: &str) -> Result<PathBuf, SecretError> {
    let trimmed = profile_id.trim();
    let safe = !trimmed.is_empty()
        && trimmed == profile_id
        && trimmed.len() <= 128
        && trimmed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if !safe {
        return Err(SecretError::InvalidProfileId);
    }
    Ok(config_dir()
        .join("secrets")
        .join(format!("{trimmed}.dpapi")))
}

pub fn store_profile_api_key(profile_id: &str, key: &str) -> Result<(), SecretError> {
    FileSecretStore::at(profile_secret_path(profile_id)?).write(key.as_bytes())
}

pub fn load_profile_api_key(profile_id: &str) -> Result<Option<String>, SecretError> {
    Ok(FileSecretStore::at(profile_secret_path(profile_id)?)
        .read()?
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}

pub fn remove_profile_api_key(profile_id: &str) -> Result<(), SecretError> {
    FileSecretStore::at(profile_secret_path(profile_id)?).write(&[])
}

pub fn migrate_legacy_api_key(profile_id: &str) -> Result<(), SecretError> {
    if load_profile_api_key(profile_id)?.is_some() {
        return Ok(());
    }
    if let Some(key) = load_api_key()? {
        if !key.is_empty() {
            store_profile_api_key(profile_id, &key)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn protect(bytes: &[u8]) -> Result<Vec<u8>, SecretError> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &mut input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok.is_err() {
        return Err(SecretError::Protect("CryptProtectData failed".into()));
    }
    let slice = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let out = slice.to_vec();
    unsafe {
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(output.pbData as _));
    }
    Ok(out)
}

#[cfg(windows)]
fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, SecretError> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok.is_err() {
        return Err(SecretError::Protect("CryptUnprotectData failed".into()));
    }
    let slice = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let out = slice.to_vec();
    unsafe {
        let _ = LocalFree(windows::Win32::Foundation::HLOCAL(output.pbData as _));
    }
    Ok(out)
}

#[cfg(not(windows))]
fn protect(bytes: &[u8]) -> Result<Vec<u8>, SecretError> {
    Ok(bytes.to_vec())
}

#[cfg(not(windows))]
fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, SecretError> {
    Ok(bytes.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid_like() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    #[test]
    fn empty_key_clears_store() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aitext-secret-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
        store_api_key("abc").unwrap();
        assert_eq!(load_api_key().unwrap().as_deref(), Some("abc"));
        store_api_key("").unwrap();
        assert_eq!(load_api_key().unwrap(), None);
    }

    #[test]
    fn profile_keys_do_not_cross_between_profiles() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aitext-secret-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);

        store_profile_api_key("deepseek", "first-test-value").unwrap();
        store_profile_api_key("openai", "second-test-value").unwrap();

        assert_eq!(
            load_profile_api_key("deepseek").unwrap().as_deref(),
            Some("first-test-value")
        );
        assert_eq!(
            load_profile_api_key("openai").unwrap().as_deref(),
            Some("second-test-value")
        );
        assert_ne!(
            profile_secret_path("deepseek").unwrap(),
            profile_secret_path("openai").unwrap()
        );
    }

    #[test]
    fn legacy_key_is_copied_without_deleting_legacy_file() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aitext-secret-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
        store_api_key("legacy-test-value").unwrap();
        let legacy_path = config_dir().join("api_key.dpapi");

        migrate_legacy_api_key("imported").unwrap();

        assert_eq!(
            load_profile_api_key("imported").unwrap().as_deref(),
            Some("legacy-test-value")
        );
        assert!(legacy_path.exists());
    }

    #[test]
    fn removing_one_profile_key_keeps_another_profile_key() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aitext-secret-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
        store_profile_api_key("one", "first-test-value").unwrap();
        store_profile_api_key("two", "second-test-value").unwrap();

        remove_profile_api_key("one").unwrap();

        assert_eq!(load_profile_api_key("one").unwrap(), None);
        assert_eq!(
            load_profile_api_key("two").unwrap().as_deref(),
            Some("second-test-value")
        );
    }

    #[test]
    fn unsafe_profile_ids_do_not_create_secret_paths() {
        assert!(profile_secret_path("").is_err());
        assert!(profile_secret_path("../outside").is_err());
        assert!(profile_secret_path("nested\\path").is_err());
    }
}
