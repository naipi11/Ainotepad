use std::path::PathBuf;

use crate::config::config_dir;

#[derive(Debug)]
pub enum SecretError {
    Io(String),
    Protect(String),
}

pub trait SecretStore {
    fn write(&self, bytes: &[u8]) -> Result<(), SecretError>;
    fn read(&self) -> Result<Option<Vec<u8>>, SecretError>;
}

pub struct FileSecretStore {
    path: PathBuf,
}

impl FileSecretStore {
    pub fn default_store() -> Self {
        Self {
            path: config_dir().join("api_key.dpapi"),
        }
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

#[cfg(windows)]
fn protect(bytes: &[u8]) -> Result<Vec<u8>, SecretError> {
    use windows::Win32::Foundation::LocalFree;
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
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
        CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
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
}
