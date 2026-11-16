//! `holden approve` (slice-7 P4): the owner signs the host's promises and
//! cognitions with the OWNER key from the owner's key store.
//!
//! Only the owner runs it. It reads the proposed cognitions from the host's
//! `.holden/cognitions/`, checks that each is its own content hash and
//! rests on a promise that is verbatim in the planning document, signs each
//! (cognition, promise) pair with the owner key, writes the approval records
//! next to them, publishes the owner's public key in the owner's key store
//! (the only trust anchor admission uses), and re-admits every cognition
//! against that published key before it reports success.
//!
//! When the owner's key store holds no owner key yet, the key is created
//! from the OS random source only when the owner asks (`create`), so the
//! first approval is still a single command.

use std::path::{Path, PathBuf};

use holden_cognition::{Approval, OwnerTrust, adapters::read_promises};
use holden_seal::{FileKeyStore, KeyMaterial, KeyName, KeyStore, KeyStoreError, OwnerKey};

use crate::host::{HostConfig, approval_file, read_cognitions, verify_promise_proposals};
use crate::util::{json_bytes, write_if_changed};
use crate::{Error, refused};

/// The owner key's name in the owner's key store.
pub const OWNER_KEY: &str = "owner";

/// One signed approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signed {
    /// The promise, in the owner's words.
    pub promise: String,
    /// The cognition id, hex.
    pub cognition: String,
    /// The approval file written.
    pub file: PathBuf,
    /// The signature, hex.
    pub signature: String,
}

/// What an approval run did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApproveReport {
    /// The owner's public key, hex.
    pub owner_public: String,
    /// Whether this run created the owner key.
    pub created_owner_key: bool,
    /// Where the public key is published.
    pub published: PathBuf,
    /// Each approval.
    pub signed: Vec<Signed>,
}

/// The named throwaway/test keychain within an owner public-key directory.
/// Production configurations without this named store use the user's default
/// keychain. The legacy `owner.key` path is never consulted on macOS.
#[cfg(target_os = "macos")]
#[must_use]
pub fn owner_keychain_path(owner_store: &Path) -> PathBuf {
    owner_store.join("owner-signing.keychain")
}

fn owner_key_from_store(store: &dyn KeyStore, create: bool) -> Result<(OwnerKey, bool), Error> {
    let name = KeyName::new(OWNER_KEY).map_err(|e| refused(e.to_string()))?;
    match OwnerKey::load(store, &name) {
        Ok(k) => Ok((k, false)),
        Err(
            KeyStoreError::NotFound { .. }
            | KeyStoreError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            },
        ) => {
            if !create {
                return Err(refused(
                    "the owner's key store holds no owner key; run holden approve with --create-owner-key once",
                ));
            }
            let bytes = holden_seal::random_bytes::<32>()
                .map_err(|_| refused("the OS random source failed"))?;
            store
                .put(&name, &KeyMaterial::from_bytes(bytes))
                .map_err(|e| refused(e.to_string()))?;
            let k = OwnerKey::load(store, &name).map_err(|e| refused(e.to_string()))?;
            Ok((k, true))
        }
        Err(e) => Err(refused(e.to_string())),
    }
}

/// Resolve only through macOS Keychain. `owner_store` remains the directory
/// for the owner's published public key; no plain owner-key file is opened.
#[cfg(target_os = "macos")]
fn require_owner_terminal() -> Result<(), Error> {
    use std::io::IsTerminal;

    if std::io::stdin().is_terminal() {
        Ok(())
    } else {
        Err(refused(
            "owner presence required: run this owner command from the owner's terminal",
        ))
    }
}

#[cfg(target_os = "macos")]
fn owner_os_store(owner_store: &Path) -> Result<holden_seal::KeychainKeyStore, Error> {
    use holden_seal::KeychainKeyStore;

    // Preserve the established private-directory permission checks for the
    // public owner.pub file without reading its former owner.key sibling.
    FileKeyStore::open(owner_store).map_err(|e| refused(e.to_string()))?;
    let named = owner_keychain_path(owner_store);
    let mut store = if named.exists() {
        KeychainKeyStore::open(&named)
    } else {
        KeychainKeyStore::default_keychain()
    }
    .map_err(|e| refused(e.to_string()))?;
    // A named keychain is the noninteractive test access path. Its password is
    // always a throwaway test value supplied by that fixture; the real owner
    // key and real login keychain are never used by the tests.
    if named.exists()
        && let Some(password) = std::env::var_os("HOLDEN_TEST_OWNER_KEYCHAIN_PASSWORD")
    {
        let password = password
            .into_string()
            .map_err(|_| refused("the test keychain password is not UTF-8"))?;
        store
            .unlock(&password)
            .map_err(|e| refused(e.to_string()))?;
    }
    Ok(store)
}

#[cfg(target_os = "macos")]
fn owner_key(owner_store: &Path, create: bool) -> Result<(OwnerKey, bool), Error> {
    require_owner_terminal()?;
    let store = owner_os_store(owner_store)?;
    owner_key_from_store(&store, create)
}

#[cfg(target_os = "linux")]
fn require_owner_terminal() -> Result<(), Error> {
    use std::io::IsTerminal;

    if std::io::stdin().is_terminal() {
        Ok(())
    } else {
        Err(refused(
            "owner presence required: run this owner command from the owner's terminal",
        ))
    }
}

#[cfg(target_os = "linux")]
fn owner_os_store(owner_store: &Path) -> Result<holden_seal::LinuxPassphraseKeyStore, Error> {
    FileKeyStore::open(owner_store).map_err(|e| refused(e.to_string()))?;
    holden_seal::LinuxPassphraseKeyStore::open(owner_store).map_err(|e| refused(e.to_string()))
}

#[cfg(target_os = "linux")]
fn owner_key(owner_store: &Path, create: bool) -> Result<(OwnerKey, bool), Error> {
    require_owner_terminal()?;
    let store = owner_os_store(owner_store)?;
    owner_key_from_store(&store, create)
}

/// Publish only after the migrated key produces the same signature, then
/// remove the legacy file. Every failure before the final removal leaves the
/// legacy custody untouched.
fn publish_and_remove_legacy(
    owner_store: &Path,
    legacy: &OwnerKey,
    owner: &OwnerKey,
) -> Result<PathBuf, Error> {
    const PROOF: &[u8] = b"holden owner-key migration custody proof";
    if legacy.public() != owner.public() || legacy.sign(PROOF) != owner.sign(PROOF) {
        return Err(refused("the migrated owner key does not sign identically"));
    }
    let published = OwnerTrust::of_owner(owner)
        .publish(owner_store)
        .map_err(|e| refused(format!("publishing the owner key: {e}")))?;
    let legacy_path = owner_store.join("owner.key");
    std::fs::remove_file(&legacy_path).map_err(|e| Error::Io {
        path: legacy_path,
        kind: e.kind(),
    })?;
    Ok(published)
}

/// OWNER-ONLY one-time migration into macOS Keychain. The old file is read
/// only by this explicitly invoked owner command; no agent may execute it for
/// the real owner. Existing Keychain custody must have the same public key.
///
/// # Errors
/// [`Error`] when owner presence, either store, key agreement or publication
/// fails.
#[cfg(target_os = "macos")]
pub fn migrate_owner_key(owner_store: &Path) -> Result<PathBuf, Error> {
    require_owner_terminal()?;
    let legacy_store = FileKeyStore::open(owner_store).map_err(|e| refused(e.to_string()))?;
    let name = KeyName::new(OWNER_KEY).map_err(|e| refused(e.to_string()))?;
    let legacy_material = legacy_store
        .get(&name)
        .map_err(|e| refused(e.to_string()))?;
    let legacy = OwnerKey::from_material(&legacy_material);
    let store = owner_os_store(owner_store)?;
    match OwnerKey::load(&store, &name) {
        Ok(existing) if existing.public() == legacy.public() => {}
        Ok(_) => {
            return Err(refused(
                "the Keychain already contains a different owner key; refusing to replace it",
            ));
        }
        Err(KeyStoreError::NotFound { .. }) => store
            .put(&name, &legacy_material)
            .map_err(|e| refused(e.to_string()))?,
        Err(e) => return Err(refused(e.to_string())),
    }
    let owner = OwnerKey::load(&store, &name).map_err(|e| refused(e.to_string()))?;
    publish_and_remove_legacy(owner_store, &legacy, &owner)
}

/// OWNER-ONLY one-time migration into the Linux passphrase-sealed store. The
/// old file is read only by this explicitly invoked owner command; no agent
/// may execute it for the real owner. Existing custody must have the same
/// public key.
///
/// # Errors
/// [`Error`] when owner presence, either store, key agreement or publication
/// fails.
#[cfg(target_os = "linux")]
pub fn migrate_owner_key(owner_store: &Path) -> Result<PathBuf, Error> {
    require_owner_terminal()?;
    let legacy_store = FileKeyStore::open(owner_store).map_err(|e| refused(e.to_string()))?;
    let name = KeyName::new(OWNER_KEY).map_err(|e| refused(e.to_string()))?;
    let legacy_material = legacy_store
        .get(&name)
        .map_err(|e| refused(e.to_string()))?;
    let legacy = OwnerKey::from_material(&legacy_material);
    let store = owner_os_store(owner_store)?;
    match OwnerKey::load(&store, &name) {
        Ok(existing) if existing.public() == legacy.public() => {}
        Ok(_) => {
            return Err(refused(
                "the sealed key store already contains a different owner key; refusing to replace it",
            ));
        }
        Err(
            KeyStoreError::NotFound { .. }
            | KeyStoreError::Io {
                kind: std::io::ErrorKind::NotFound,
                ..
            },
        ) => store
            .put(&name, &legacy_material)
            .map_err(|e| refused(e.to_string()))?,
        Err(e) => return Err(refused(e.to_string())),
    }
    let owner = OwnerKey::load(&store, &name).map_err(|e| refused(e.to_string()))?;
    publish_and_remove_legacy(owner_store, &legacy, &owner)
}

/// Approve the host's cognitions with the owner key (see the module docs).
///
/// # Errors
/// [`Error`] when the host's files, the owner's key store or a re-admission
/// refuses.
pub fn approve(host: &Path, owner_store: &Path, create: bool) -> Result<ApproveReport, Error> {
    let _config = HostConfig::read(host)?;
    let client = read_promises(host).map_err(|e| refused(e.to_string()))?;
    verify_promise_proposals(host, &client)?;
    let cognitions = read_cognitions(host)?;
    if cognitions.is_empty() {
        return Err(refused(
            "the host has no proposed cognition; run holden embed first",
        ));
    }
    for c in &cognitions {
        if c.computed_id() != c.id {
            return Err(refused(format!(
                "cognition {} is not its content hash",
                c.id.to_hex()
            )));
        }
        let Some((document, promise)) = client.spans().into_iter().find(|(_, p)| p.id == c.promise)
        else {
            return Err(refused(format!(
                "cognition {} rests on no promise of the planning documents",
                c.id.to_hex()
            )));
        };
        promise
            .verify_against(document)
            .map_err(|e| refused(e.to_string()))?;
    }
    let (owner, created_owner_key) = owner_key(owner_store, create)?;
    let mut approvals = Vec::new();
    let mut signed = Vec::new();
    for c in &cognitions {
        let a = Approval::sign(&owner, c);
        let file = approval_file(host, &c.id.to_hex());
        write_if_changed(&file, &json_bytes(&a)?, 0o644)?;
        let promise = client
            .spans()
            .into_iter()
            .find(|(_, p)| p.id == c.promise)
            .map(|(_, p)| p.text.clone())
            .unwrap_or_default();
        signed.push(Signed {
            promise,
            cognition: c.id.to_hex(),
            file,
            signature: a.signature.clone(),
        });
        approvals.push(a);
    }
    let published = OwnerTrust::of_owner(&owner)
        .publish(owner_store)
        .map_err(|e| refused(format!("publishing the owner key: {e}")))?;
    let trust = OwnerTrust::read_published(owner_store).map_err(|e| refused(e.to_string()))?;
    let (admitted, refusals) = client.admit(&cognitions, &approvals, &trust);
    if !refusals.is_empty() || admitted.len() != cognitions.len() {
        return Err(refused(format!(
            "{} cognition(s) were not admitted after signing",
            refusals.len()
        )));
    }
    Ok(ApproveReport {
        owner_public: hex::encode(owner.public().0),
        created_owner_key,
        published,
        signed,
    })
}
