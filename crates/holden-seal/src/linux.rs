//! Linux owner-key custody without a required desktop service.
//!
//! Each key is sealed at rest with XChaCha20-Poly1305 under a fresh salt and
//! nonce. Argon2id derives the encryption key from a passphrase read directly
//! from the controlling terminal by `rpassword`, with echo disabled. There is
//! deliberately no stdin, environment-variable or passphrase-file fallback.

use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::key::{KeyMaterial, os_random};
use crate::keystore::{
    FileKeyStore, KEY_FILE_MODE, KeyName, KeyStore, KeyStoreError, check_dir, check_owner_and_mode,
    io_err,
};

const FORMAT_VERSION: u8 = 1;
const MAX_SEALED_BYTES: u64 = 16 * 1024;
const AAD: &[u8] = b"holden-mcp/linux-owner-key/v1";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SealedKeyFile {
    version: u8,
    salt: String,
    nonce: String,
    ciphertext: String,
}

/// A Linux key store whose private keys are passphrase-sealed at rest.
#[derive(Debug, Clone)]
pub struct LinuxPassphraseKeyStore {
    dir: PathBuf,
}

impl LinuxPassphraseKeyStore {
    /// Open the store at `dir`, applying the same 0700 ownership checks as the
    /// plain file store. Sealed siblings use the `.sealed.json` suffix; the
    /// legacy `.key` file is never consulted.
    ///
    /// # Errors
    /// [`KeyStoreError`] when the directory is absent and cannot be created,
    /// or fails its ownership and permission checks.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, KeyStoreError> {
        let dir = dir.into();
        FileKeyStore::open(dir.clone())?;
        Ok(Self { dir })
    }

    /// The sealed path for `name`.
    #[must_use]
    pub fn key_path(&self, name: &KeyName) -> PathBuf {
        self.dir.join(format!("{}.sealed.json", name.as_str()))
    }

    fn passphrase(path: &Path) -> Result<Zeroizing<String>, KeyStoreError> {
        rpassword::prompt_password("Owner passphrase: ")
            .map(Zeroizing::new)
            .map_err(|_| KeyStoreError::InteractiveTerminalRequired {
                path: path.to_path_buf(),
            })
    }

    fn encryption_key(
        passphrase: &str,
        salt: &[u8],
        path: &Path,
    ) -> Result<Zeroizing<[u8; 32]>, KeyStoreError> {
        if passphrase.is_empty() {
            return Err(KeyStoreError::SealedKey {
                path: path.to_path_buf(),
            });
        }
        let mut key = Zeroizing::new([0u8; 32]);
        Argon2::default()
            .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
            .map_err(|_| KeyStoreError::SealedKey {
                path: path.to_path_buf(),
            })?;
        Ok(key)
    }

    pub(crate) fn seal_with_passphrase(
        &self,
        name: &KeyName,
        key: &KeyMaterial,
        passphrase: &str,
    ) -> Result<(), KeyStoreError> {
        check_dir(&self.dir)?;
        let path = self.key_path(name);
        let mut salt = [0u8; 16];
        let mut nonce = [0u8; 24];
        os_random(&mut salt).map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        os_random(&mut nonce).map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        let encryption_key = Self::encryption_key(passphrase, &salt, &path)?;
        let cipher = XChaCha20Poly1305::new_from_slice(encryption_key.as_ref())
            .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: key.expose(),
                    aad: AAD,
                },
            )
            .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        let wire = SealedKeyFile {
            version: FORMAT_VERSION,
            salt: hex::encode(salt),
            nonce: hex::encode(nonce),
            ciphertext: hex::encode(ciphertext),
        };
        let bytes = serde_json::to_vec(&wire)
            .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(KEY_FILE_MODE)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    KeyStoreError::AlreadyExists { path: path.clone() }
                } else {
                    io_err(&path)(e)
                }
            })?;
        file.set_permissions(Permissions::from_mode(KEY_FILE_MODE))
            .map_err(io_err(&path))?;
        file.write_all(&bytes).map_err(io_err(&path))?;
        file.sync_all().map_err(io_err(&path))?;
        Ok(())
    }

    fn unseal_with_passphrase(
        &self,
        name: &KeyName,
        passphrase: &str,
    ) -> Result<KeyMaterial, KeyStoreError> {
        check_dir(&self.dir)?;
        let path = self.key_path(name);
        let before = fs::symlink_metadata(&path).map_err(io_err(&path))?;
        if !before.file_type().is_file() {
            return Err(KeyStoreError::NotARegularFile { path });
        }
        check_owner_and_mode(&path, &before)?;
        let file = File::open(&path).map_err(io_err(&path))?;
        let opened = file.metadata().map_err(io_err(&path))?;
        if opened.dev() != before.dev() || opened.ino() != before.ino() {
            return Err(KeyStoreError::ChangedDuringLoad { path });
        }
        check_owner_and_mode(&path, &opened)?;
        if opened.len() > MAX_SEALED_BYTES {
            return Err(KeyStoreError::SealedKey { path });
        }
        let mut bytes = Vec::new();
        file.take(MAX_SEALED_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(io_err(&path))?;
        if bytes.len() as u64 > MAX_SEALED_BYTES {
            return Err(KeyStoreError::SealedKey { path });
        }
        let wire: SealedKeyFile = serde_json::from_slice(&bytes)
            .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        if wire.version != FORMAT_VERSION {
            return Err(KeyStoreError::SealedKey { path });
        }
        let salt =
            hex::decode(&wire.salt).map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        let nonce = hex::decode(&wire.nonce)
            .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        let mut ciphertext = hex::decode(&wire.ciphertext)
            .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        if salt.len() != 16 || nonce.len() != 24 {
            return Err(KeyStoreError::SealedKey { path });
        }
        let mut encryption_key = Self::encryption_key(passphrase, &salt, &path)?;
        let cipher = XChaCha20Poly1305::new_from_slice(encryption_key.as_ref())
            .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?;
        let plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(&nonce),
                    Payload {
                        msg: &ciphertext,
                        aad: AAD,
                    },
                )
                .map_err(|_| KeyStoreError::SealedKey { path: path.clone() })?,
        );
        ciphertext.zeroize();
        encryption_key.zeroize();
        let len = plaintext.len() as u64;
        let key: [u8; 32] = plaintext
            .as_slice()
            .try_into()
            .map_err(|_| KeyStoreError::BadLength { path, len })?;
        Ok(KeyMaterial::from_bytes(key))
    }
}

impl KeyStore for LinuxPassphraseKeyStore {
    fn put(&self, name: &KeyName, key: &KeyMaterial) -> Result<(), KeyStoreError> {
        let path = self.key_path(name);
        let passphrase = Self::passphrase(&path)?;
        self.seal_with_passphrase(name, key, &passphrase)
    }

    fn get(&self, name: &KeyName) -> Result<KeyMaterial, KeyStoreError> {
        let path = self.key_path(name);
        if let Err(error) = fs::symlink_metadata(&path) {
            return if error.kind() == std::io::ErrorKind::NotFound {
                Err(KeyStoreError::NotFound { path })
            } else {
                Err(io_err(&path)(error))
            };
        }
        // Resolve the terminal before touching the sealed bytes. A caller with
        // no controlling TTY therefore cannot turn this into an offline read.
        let passphrase = Self::passphrase(&path)?;
        self.unseal_with_passphrase(name, &passphrase)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::DirBuilder;
    use std::os::unix::fs::DirBuilderExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "holden-linux-keystore-{tag}-{}-{unique}",
            std::process::id()
        ));
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)
            .unwrap();
        root
    }

    #[test]
    fn sealed_roundtrip_uses_argon2id_and_xchacha20poly1305() {
        let root = scratch("roundtrip");
        let store = LinuxPassphraseKeyStore::open(&root).unwrap();
        let name = KeyName::new("owner").unwrap();
        let key = KeyMaterial::generate().unwrap();
        store
            .seal_with_passphrase(&name, &key, "throwaway-test-passphrase")
            .unwrap();
        let bytes = fs::read(store.key_path(&name)).unwrap();
        assert!(!bytes.windows(32).any(|window| window == key.expose()));
        let reopened = store
            .unseal_with_passphrase(&name, "throwaway-test-passphrase")
            .unwrap();
        assert!(key.ct_eq(&reopened));
        assert!(matches!(
            store.unseal_with_passphrase(&name, "wrong-throwaway-passphrase"),
            Err(KeyStoreError::SealedKey { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    /// Slice-12b P2. The sealed bytes are genuinely present, but a non-TTY
    /// caller cannot obtain a passphrase and receives a typed refusal before
    /// any key material is returned.
    /// Fail-without mutant: return the plaintext without the TTY unseal.
    #[test]
    fn unseal_requires_interactive_tty() {
        let root = scratch("non-tty");
        let store = LinuxPassphraseKeyStore::open(&root).unwrap();
        let name = KeyName::new("owner").unwrap();
        store
            .seal_with_passphrase(
                &name,
                &KeyMaterial::generate().unwrap(),
                "throwaway-test-passphrase",
            )
            .unwrap();
        assert!(store.key_path(&name).is_file());
        assert!(matches!(
            store.get(&name),
            Err(KeyStoreError::InteractiveTerminalRequired { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }
}
