//! Slice-17 P6: disposable-keychain owner-key migration custody.

#![cfg(target_os = "macos")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use holden_seal::{FileKeyStore, KeyMaterial, KeyName, KeyStore, KeychainKeyStore, OwnerKey};

const PASSWORD: &str = "throwaway-migration-keychain-password";

struct TempRoot(PathBuf);

impl TempRoot {
    fn new() -> Self {
        let root = PathBuf::from("/tmp")
            .join(format!("holden-cli-owner-migration-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn migrate(binary: &Path, store: &Path) -> std::process::Output {
    Command::new("/usr/bin/script")
        .args(["-q", "/dev/null"])
        .arg(binary)
        .args(["migrate-owner-key", "--owner-store"])
        .arg(store)
        .env("HOLDEN_TEST_OWNER_KEYCHAIN_PASSWORD", PASSWORD)
        .output()
        .unwrap()
}

fn put_legacy(store: &FileKeyStore, material: &KeyMaterial) {
    store
        .put(&KeyName::new("owner").unwrap(), material)
        .unwrap();
}

/// Fail-without mutant: delete `owner.key` before checking the migrated
/// signature. The different-key refusal then wrongly deletes the legacy file.
#[test]
fn migrate_removes_legacy_key_after_verify() {
    let scratch = TempRoot::new();
    let root = scratch.path().to_path_buf();
    let store_path = root.join("owner-keys");
    let legacy_store = FileKeyStore::open(&store_path).unwrap();
    let keychain_path = holden_embed::approve::owner_keychain_path(&store_path);
    assert!(keychain_path.starts_with(&root));
    assert_ne!(
        keychain_path.file_name().unwrap().to_string_lossy(),
        "login.keychain-db"
    );

    // A different already-custodied key is a failed verification: no removal.
    let legacy_material = KeyMaterial::from_bytes(holden_seal::random_bytes::<32>().unwrap());
    put_legacy(&legacy_store, &legacy_material);
    let keychain = KeychainKeyStore::create(&keychain_path, PASSWORD).unwrap();
    let other = KeyMaterial::from_bytes(holden_seal::random_bytes::<32>().unwrap());
    keychain
        .put(&KeyName::new("owner").unwrap(), &other)
        .unwrap();
    let failed = migrate(Path::new(env!("CARGO_BIN_EXE_holden")), &store_path);
    assert_eq!(failed.status.code(), Some(1));
    assert!(store_path.join("owner.key").is_file());

    // Recreate only the disposable named keychain, then migrate successfully.
    drop(keychain);
    fs::remove_file(&keychain_path).unwrap();
    let keychain = KeychainKeyStore::create(&keychain_path, PASSWORD).unwrap();
    drop(keychain);
    let succeeded = migrate(Path::new(env!("CARGO_BIN_EXE_holden")), &store_path);
    assert!(
        succeeded.status.success(),
        "{}",
        String::from_utf8_lossy(&succeeded.stderr)
    );
    assert!(!store_path.join("owner.key").exists());
    let legacy = OwnerKey::from_material(&legacy_material);
    let published = fs::read_to_string(store_path.join("owner.pub")).unwrap();
    assert_eq!(published.trim(), hex::encode(legacy.public().0));
    fs::remove_dir_all(&root).unwrap();
}
