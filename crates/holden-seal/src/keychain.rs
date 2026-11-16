//! macOS Keychain implementation of [`KeyStore`]. Keys are generic-password
//! items whose secret data is exactly 32 bytes. No operation prints or logs
//! their bytes. Callers may select a named keychain file; otherwise the user's
//! default keychain is used. Automatic Security-framework UI is disabled for
//! every open, read and write, so a locked or inaccessible store is an error,
//! never a GUI wait.

use std::fmt;
use std::path::{Path, PathBuf};

use security_framework::base::Error;
use security_framework::os::macos::keychain::{CreateOptions, SecKeychain};

use crate::key::KeyMaterial;
use crate::keystore::{KeyName, KeyStore, KeyStoreError};

const SERVICE: &str = "holden-mcp.owner-signing";
const ITEM_NOT_FOUND: i32 = -25_300;
const DUPLICATE_ITEM: i32 = -25_299;

/// A macOS Keychain-backed key store.
#[derive(Clone)]
pub struct KeychainKeyStore {
    keychain: SecKeychain,
    path: Option<PathBuf>,
}

impl fmt::Debug for KeychainKeyStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeychainKeyStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl KeychainKeyStore {
    fn error_path(path: Option<&Path>) -> PathBuf {
        path.map_or_else(|| PathBuf::from("<default keychain>"), Path::to_path_buf)
    }

    fn error(path: Option<&Path>, operation: &'static str, error: Error) -> KeyStoreError {
        let path = Self::error_path(path);
        match error.code() {
            ITEM_NOT_FOUND => KeyStoreError::NotFound { path },
            DUPLICATE_ITEM => KeyStoreError::AlreadyExists { path },
            code => KeyStoreError::Keychain {
                path,
                operation,
                code,
            },
        }
    }

    fn disable_ui(
        &self,
    ) -> Result<security_framework::os::macos::keychain::KeychainUserInteractionLock, KeyStoreError>
    {
        SecKeychain::disable_user_interaction()
            .map_err(|e| Self::error(self.path(), "disable automatic Keychain UI", e))
    }

    /// Use the user's default keychain. Automatic UI remains disabled during
    /// operations; an inaccessible item is refused rather than prompting.
    ///
    /// # Errors
    /// [`KeyStoreError::Keychain`] if Security.framework cannot resolve it.
    pub fn default_keychain() -> Result<Self, KeyStoreError> {
        let keychain =
            SecKeychain::default().map_err(|e| Self::error(None, "open default keychain", e))?;
        Ok(Self {
            keychain,
            path: None,
        })
    }

    /// Open the named keychain at `path`, without creating it.
    ///
    /// # Errors
    /// [`KeyStoreError::Keychain`] or [`KeyStoreError::NotFound`] when the
    /// named store cannot be opened.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, KeyStoreError> {
        let path = path.as_ref();
        let _ui = SecKeychain::disable_user_interaction()
            .map_err(|e| Self::error(Some(path), "disable automatic Keychain UI", e))?;
        let keychain = SecKeychain::open(path)
            .map_err(|e| Self::error(Some(path), "open named keychain", e))?;
        Ok(Self {
            keychain,
            path: Some(path.to_path_buf()),
        })
    }

    /// Unlock a named keychain with its caller-supplied password. No UI is
    /// enabled; a wrong password is an ordinary typed refusal.
    ///
    /// # Errors
    /// [`KeyStoreError::Keychain`] when Security.framework refuses the unlock.
    pub fn unlock(&mut self, password: &str) -> Result<(), KeyStoreError> {
        let _ui = SecKeychain::disable_user_interaction()
            .map_err(|e| Self::error(self.path(), "disable automatic Keychain UI", e))?;
        self.keychain
            .unlock(Some(password))
            .map_err(|e| Self::error(self.path(), "unlock named keychain", e))
    }

    /// Create a named keychain at a path that does not exist, using `password`
    /// supplied by the caller and never prompting. It is returned unlocked.
    /// Tests supply a throwaway password and delete the whole named keychain.
    ///
    /// # Errors
    /// [`KeyStoreError::AlreadyExists`] or [`KeyStoreError::Keychain`] when
    /// Security.framework refuses creation.
    pub fn create(path: impl AsRef<Path>, password: &str) -> Result<Self, KeyStoreError> {
        let path = path.as_ref();
        if path.exists() {
            return Err(KeyStoreError::AlreadyExists {
                path: path.to_path_buf(),
            });
        }
        let _ui = SecKeychain::disable_user_interaction()
            .map_err(|e| Self::error(Some(path), "disable automatic Keychain UI", e))?;
        let keychain = CreateOptions::new()
            .password(password)
            .prompt_user(false)
            .create(path)
            .map_err(|e| Self::error(Some(path), "create named keychain", e))?;
        Ok(Self {
            keychain,
            path: Some(path.to_path_buf()),
        })
    }

    /// The named keychain path, or `None` for the user's default keychain.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl KeyStore for KeychainKeyStore {
    fn put(&self, name: &KeyName, key: &KeyMaterial) -> Result<(), KeyStoreError> {
        let _ui = self.disable_ui()?;
        self.keychain
            .add_generic_password(SERVICE, name.as_str(), key.expose())
            .map_err(|e| Self::error(self.path(), "add keychain item", e))
    }

    fn get(&self, name: &KeyName) -> Result<KeyMaterial, KeyStoreError> {
        let _ui = self.disable_ui()?;
        let (password, _item) = self
            .keychain
            .find_generic_password(SERVICE, name.as_str())
            .map_err(|e| Self::error(self.path(), "read keychain item", e))?;
        let len = password.as_ref().len() as u64;
        let bytes: [u8; 32] =
            password
                .as_ref()
                .try_into()
                .map_err(|_| KeyStoreError::BadLength {
                    path: Self::error_path(self.path()),
                    len,
                })?;
        Ok(KeyMaterial::from_bytes(bytes))
    }
}
