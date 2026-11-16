//! holden-cognition: planning evidence → verbatim promises → candidate
//! situated cognitions → owner approval records (docs/ARCHITECTURE.md §3.1).
//!
//! - [`promise`]: promises as byte spans of a planning document, with the
//!   source file's SHA-256; the text is sliced from the source bytes.
//! - [`cognition`]: deterministic candidate cognitions from the doctrine's
//!   generator rules, optionally shaped by a forge pack's domain overlay.
//! - [`approval`]: Ed25519 owner approvals; only an approved cognition can
//!   host tests. Slice-7: [`OwnerTrust`] takes the trusted owner key only
//!   from the owner's key store, and [`admit_approved`] admits a host's
//!   approved cognitions against it.
//! - [`forge_pack`]: the typed loader for a forge export, read purely as
//!   files (no crate dependency on the forge).

pub mod adapters;
pub mod approval;
pub mod cognition;
pub mod forge_pack;
pub mod promise;

pub use adapters::{
    AdapterError, AdapterKind, AdapterLimitation, ClientPromises, DiscoveryKind, DocumentPromises,
    HoldoutCheckpoint, PlanningAdapter, WritebackOptions, WritebackOutcome, detect, read_promises,
    write_back,
};
pub use approval::{
    Approval, ApprovalError, HostableCognition, OWNER_PUBLIC_FILE, OwnerTrust, admit_approved,
};
pub use cognition::{SituatedCognition, candidates};
pub use forge_pack::{DomainOverlay, LoadedForge, LoadedPack, load_forge, load_pack_file};
pub use promise::{ContentId, PlanningDocument, Promise, PromiseError, extract_promises};

/// Paths of the committed synthetic fixture project, relative to this
/// crate's manifest directory.
pub mod fixture {
    /// The fixture project directory.
    pub const PROJECT: &str = "fixtures/project-synthetic";
    /// Its planning document.
    pub const PLANNING: &str = "fixtures/project-synthetic/PLANNING.md";
    /// The holdout-engineering pack name.
    pub const PACK_NAME: &str = "holdout-engineering";
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    use holden_canon::Doctrine;
    use holden_seal::{FileKeyStore, KeyMaterial, KeyName, KeyStore, OwnerKey};

    use super::*;

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn planning() -> PlanningDocument {
        PlanningDocument::read(&crate_dir().join(fixture::PLANNING), "PLANNING.md").unwrap()
    }

    fn forge_dir() -> PathBuf {
        crate_dir().join("../../modules/holden-forge")
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "holden-cognition-test-{name}-{}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// An owner key put into, then loaded from, a 0700 file key store.
    fn owner_from_store(name: &str, seed: u8) -> OwnerKey {
        let dir = scratch(&format!("keys-{name}")).join("store");
        let store = FileKeyStore::open(&dir).unwrap();
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let key_name = KeyName::new("owner").unwrap();
        store
            .put(&key_name, &KeyMaterial::from_bytes([seed; 32]))
            .unwrap();
        OwnerKey::load(&store, &key_name).unwrap()
    }

    fn first_candidate() -> (PlanningDocument, Promise, SituatedCognition) {
        let doc = planning();
        let promise = extract_promises(&doc).unwrap().remove(0);
        let c = candidates(&Doctrine::holden(), &promise, None).remove(0);
        (doc, promise, c)
    }

    /// slice-3 P2: a quote is accepted only when its bytes are exactly the
    /// source bytes at its offsets. Whitespace or case variants of the true
    /// text are refused, both as a new quote and as a stored promise.
    #[test]
    fn promise_quote_must_match_source_bytes() {
        let doc = planning();
        let promises = extract_promises(&doc).unwrap();
        assert!(!promises.is_empty());
        for p in &promises {
            let exact = Promise::from_quote(&doc, p.start, p.end, &p.text).unwrap();
            assert_eq!(&exact, p);
            let variants = [
                format!("{} ", p.text),
                format!(" {}", p.text),
                p.text.replacen(' ', "  ", 1),
                p.text.replacen(' ', "\t", 1),
                p.text.to_lowercase(),
            ];
            for v in &variants {
                assert_ne!(v, &p.text);
                assert_eq!(
                    Promise::from_quote(&doc, p.start, p.end, v),
                    Err(PromiseError::QuoteDiffers {
                        start: p.start,
                        end: p.end
                    }),
                    "variant {v:?} must be refused"
                );
                let mut stored = p.clone();
                stored.text.clone_from(v);
                stored.id = promise::promise_id(&stored.source, stored.start, stored.end, v);
                assert!(
                    matches!(
                        stored.verify_against(&doc),
                        Err(PromiseError::QuoteDiffers { .. })
                    ),
                    "stored variant {v:?} must be refused"
                );
            }
        }
    }

    /// slice-3 P2: every promise carries the SHA-256 of its source file and
    /// byte offsets whose bytes are its text; a changed source is refused.
    #[test]
    fn promise_carries_source_hash_and_offsets() {
        let doc = planning();
        let promises = extract_promises(&doc).unwrap();
        assert_eq!(promises.len(), 4, "the fixture states four promises");
        for p in &promises {
            assert_eq!(p.source, doc.sha256());
            let (s, e) = (
                usize::try_from(p.start).unwrap(),
                usize::try_from(p.end).unwrap(),
            );
            assert_eq!(&doc.bytes[s..e], p.text.as_bytes());
            assert_eq!(
                p.id,
                promise::promise_id(&p.source, p.start, p.end, &p.text)
            );
            p.verify_against(&doc).unwrap();
            assert!(!p.text.contains("PROMISE"));
        }
        assert_eq!(
            promises[0].text,
            "A saved note is never lost, even if the app closes halfway through saving."
        );
        assert!(promises.iter().all(|p| !p.text.contains("undo")));
        let mut changed = doc.clone();
        changed.bytes[0] ^= 0x01;
        assert!(matches!(
            promises[0].verify_against(&changed),
            Err(PromiseError::SourceDiffers { .. })
        ));
        assert!(matches!(
            Promise::slice(&doc, 10, doc.bytes.len() as u64 + 1),
            Err(PromiseError::OutOfRange { .. })
        ));
    }

    /// slice-3 P3: the same inputs give the same candidates; every id is a
    /// content hash; every candidate cites the root and axes that made it.
    #[test]
    fn candidates_are_deterministic() {
        let doctrine = Doctrine::holden();
        let doc = planning();
        let forge = load_forge(&forge_dir(), fixture::PACK_NAME).unwrap();
        let overlay = DomainOverlay::from_pack(&forge.pack);
        let per_promise: usize = doctrine
            .generators
            .iter()
            .map(|g| g.constituencies.len())
            .sum();
        for p in extract_promises(&doc).unwrap() {
            for o in [None, Some(&overlay)] {
                let a = candidates(&doctrine, &p, o);
                let b = candidates(&doctrine, &p, o);
                assert_eq!(a, b);
                assert_eq!(a.len(), per_promise);
                for c in &a {
                    assert_eq!(c.id, c.computed_id());
                    assert_eq!(c.promise, p.id);
                    assert_eq!(c.roots.len(), 1);
                    let rule = doctrine.generator(c.roots[0]).unwrap();
                    assert!(rule.constituencies.contains(&c.constituency));
                    assert_eq!(c.axes, rule.axes);
                    let axes: Vec<_> = c.circumstance.iter().map(|l| l.axis()).collect();
                    assert_eq!(axes, rule.axes);
                    assert_eq!(c.evidence.len(), 1);
                    assert_eq!(
                        (c.evidence[0].source, c.evidence[0].start, c.evidence[0].end),
                        (p.source, p.start, p.end)
                    );
                    assert!(c.goal.contains(&p.text));
                }
            }
        }
    }

    /// slice-3 P3: no approval, an approval with a signature over other
    /// bytes, and an all-zero signature are refused; only the owner's valid
    /// signature admits the cognition.
    #[test]
    fn unapproved_cognition_cannot_host_test() {
        let (doc, promise, c) = first_candidate();
        let owner = owner_from_store("unapproved", 0x31);
        let trusted = owner.public();
        assert_eq!(
            HostableCognition::admit(&doc, &promise, &c, None, &trusted),
            Err(ApprovalError::Unapproved)
        );
        let other = candidates(&Doctrine::holden(), &promise, None).remove(1);
        let mut forged = Approval::sign(&owner, &other);
        forged.cognition = c.id;
        assert_eq!(
            HostableCognition::admit(&doc, &promise, &c, Some(&forged), &trusted),
            Err(ApprovalError::BadSignature)
        );
        let mut zero = Approval::sign(&owner, &c);
        zero.signature = "00".repeat(64);
        assert_eq!(
            HostableCognition::admit(&doc, &promise, &c, Some(&zero), &trusted),
            Err(ApprovalError::BadSignature)
        );
        let mut flipped = Approval::sign(&owner, &c);
        let last = if flipped.signature.ends_with('0') {
            "1"
        } else {
            "0"
        };
        flipped.signature.replace_range(127.., last);
        assert_eq!(
            HostableCognition::admit(&doc, &promise, &c, Some(&flipped), &trusted),
            Err(ApprovalError::BadSignature)
        );
        let good = Approval::sign(&owner, &c);
        let hosted = HostableCognition::admit(&doc, &promise, &c, Some(&good), &trusted).unwrap();
        assert_eq!(hosted.cognition(), &c);
        assert_eq!(hosted.promise(), &promise);
        let mut tampered = c.clone();
        tampered.goal.push('!');
        assert_eq!(
            HostableCognition::admit(&doc, &promise, &tampered, Some(&good), &trusted),
            Err(ApprovalError::CognitionIdNotContentHash)
        );
    }

    /// slice-3 P3: an approval by another key, whether it names that key or
    /// claims the owner's, is refused.
    #[test]
    fn wrong_owner_key_is_refused() {
        let (doc, promise, c) = first_candidate();
        let owner = owner_from_store("owner-a", 0x41);
        let intruder = owner_from_store("owner-b", 0x42);
        assert_ne!(owner.public(), intruder.public());
        let by_intruder = Approval::sign(&intruder, &c);
        assert_eq!(
            HostableCognition::admit(&doc, &promise, &c, Some(&by_intruder), &owner.public()),
            Err(ApprovalError::OwnerKeyMismatch)
        );
        let mut claims_owner = by_intruder;
        claims_owner.owner = hex::encode(owner.public().0);
        assert_eq!(
            HostableCognition::admit(&doc, &promise, &c, Some(&claims_owner), &owner.public()),
            Err(ApprovalError::BadSignature)
        );
        let good = Approval::sign(&owner, &c);
        assert!(
            HostableCognition::admit(&doc, &promise, &c, Some(&good), &intruder.public()).is_err()
        );
    }

    /// A real Ed25519 key from the OS random source, put into, then loaded
    /// from, its own 0700 file key store under `name`.
    fn random_key_in_store(store_name: &str, key_name: &str) -> (FileKeyStore, OwnerKey) {
        let store = FileKeyStore::open(scratch(store_name).join("store")).unwrap();
        let name = KeyName::new(key_name).unwrap();
        let bytes = holden_seal::random_bytes::<32>().unwrap();
        store.put(&name, &KeyMaterial::from_bytes(bytes)).unwrap();
        let key = OwnerKey::load(&store, &name).unwrap();
        (store, key)
    }

    /// slice-7 P4: a host's cognitions are admitted only against the owner
    /// key from the owner's key store. The owner (a test owner key, real
    /// Ed25519, in its own temporary key store) publishes its public half
    /// there, and admission trusts only that. An agent signing approvals
    /// with its own key (from the agent's key store) is refused, whether the
    /// record names the agent's key or claims the owner's; a set where only
    /// some approvals are the owner's admits only those.
    /// Fail-without mutant: in `admit_approved`, trust the key the approval
    /// record names instead of the owner's.
    #[test]
    fn agent_signed_approval_is_refused() {
        let doc = planning();
        let promises = extract_promises(&doc).unwrap();
        let cognitions: Vec<SituatedCognition> = promises
            .iter()
            .map(|p| candidates(&Doctrine::holden(), p, None).remove(0))
            .collect();
        let (owner_store, owner) = random_key_in_store("p4-owner", "owner");
        let (_agent_store, agent) = random_key_in_store("p4-agent", "evaluator");
        assert_ne!(owner.public(), agent.public());
        // The owner publishes the public half in the owner's key store; the
        // trust is read back from there.
        OwnerTrust::of_owner(&owner)
            .publish(owner_store.dir())
            .unwrap();
        let trust = OwnerTrust::read_published(owner_store.dir()).unwrap();
        assert_eq!(trust.public(), &owner.public());

        let by_agent: Vec<Approval> = cognitions
            .iter()
            .map(|c| Approval::sign(&agent, c))
            .collect();
        let (admitted, refused) = admit_approved(&doc, &promises, &cognitions, &by_agent, &trust);
        assert!(admitted.is_empty(), "an agent-signed approval was admitted");
        assert_eq!(refused.len(), cognitions.len());
        assert!(
            refused
                .iter()
                .all(|(_, e)| *e == ApprovalError::OwnerKeyMismatch),
            "{refused:?}"
        );

        let claiming: Vec<Approval> = by_agent
            .iter()
            .cloned()
            .map(|mut a| {
                a.owner = hex::encode(owner.public().0);
                a
            })
            .collect();
        let (admitted, refused) = admit_approved(&doc, &promises, &cognitions, &claiming, &trust);
        assert!(admitted.is_empty(), "a forged owner claim was admitted");
        assert!(
            refused
                .iter()
                .all(|(_, e)| *e == ApprovalError::BadSignature),
            "{refused:?}"
        );

        let by_owner: Vec<Approval> = cognitions
            .iter()
            .map(|c| Approval::sign(&owner, c))
            .collect();
        let (admitted, refused) = admit_approved(&doc, &promises, &cognitions, &by_owner, &trust);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(admitted.len(), cognitions.len());

        let mut mixed = by_agent.clone();
        mixed[0] = by_owner[0].clone();
        let (admitted, refused) = admit_approved(&doc, &promises, &cognitions, &mixed, &trust);
        assert_eq!(admitted.len(), 1);
        assert_eq!(admitted[0].cognition().id, cognitions[0].id);
        assert_eq!(refused.len(), cognitions.len() - 1);
        // A published key is never replaced by another.
        assert!(
            OwnerTrust::of_owner(&agent)
                .publish(owner_store.dir())
                .is_err()
        );
        assert_eq!(
            OwnerTrust::read_published(owner_store.dir()).unwrap(),
            trust
        );
    }

    fn assert_forge_matches_files(dir: &Path, f: &LoadedForge) {
        let pack_file = forge_pack::pack_path(dir, fixture::PACK_NAME);
        let bytes = fs::read(&pack_file).unwrap();
        assert_eq!(f.pack.pack.to_canonical().unwrap(), bytes);
        assert_eq!(f.pack.digest, holden_canon::Sha256Digest::of(&bytes));
        assert_eq!(f.constitutions.len(), 4);
        for (c, role) in f.constitutions.iter().zip(forge_pack::Role::ALL) {
            assert_eq!(c.role, role);
            let text =
                fs::read_to_string(forge_pack::constitution_path(dir, fixture::PACK_NAME, role))
                    .unwrap();
            assert_eq!(c.render(), text);
            assert_eq!(c.pack_digest, format!("sha256:{}", f.pack.digest.to_hex()));
            assert_eq!(c.sections.len(), 9);
        }
    }

    /// slice-3 P7: the committed holdout-engineering pack and its four
    /// constitutions load losslessly through the typed file loader. When
    /// `HOLDEN_FORGE_EXPORT` names a `forge export` directory, the export
    /// loads too and equals the committed module.
    #[test]
    fn loads_forge_pack() {
        let committed = load_forge(&forge_dir(), fixture::PACK_NAME).unwrap();
        assert_forge_matches_files(&forge_dir(), &committed);
        assert_eq!(committed.pack.pack.pack_name, fixture::PACK_NAME);
        assert!(!committed.pack.pack.worldviews.is_empty());
        let overlay = DomainOverlay::from_pack(&committed.pack);
        assert_eq!(overlay.domain, committed.pack.pack.domain);
        assert!(!overlay.failures.is_empty());
        if let Some(export) = std::env::var_os("HOLDEN_FORGE_EXPORT") {
            let export = PathBuf::from(export);
            let exported = load_forge(&export, fixture::PACK_NAME).unwrap();
            assert_forge_matches_files(&export, &exported);
            assert_eq!(exported, committed);
            println!(
                "loads_forge_pack: export {} equals the committed module",
                export.display()
            );
        }
        // A pack with an unknown member is refused, naming the file.
        let dir = scratch("unknown-member");
        let bad = dir.join("PACK.json");
        let text = fs::read_to_string(forge_pack::pack_path(&forge_dir(), fixture::PACK_NAME))
            .unwrap()
            .replacen("{\n", "{\n  \"extra\": 1,\n", 1);
        fs::write(&bad, text).unwrap();
        let err = load_pack_file(&bad).unwrap_err();
        assert_eq!(err.file, bad);
    }

    /// slice-3 P7: the pack's overlay fields (domain, worldviews, signature
    /// failures) shape the candidates: a different pack gives different
    /// candidates, deterministically, and each candidate carries its own
    /// pack's domain and one of its own pack's failures.
    #[test]
    fn pack_overlay_changes_candidates() {
        let doctrine = Doctrine::holden();
        let doc = planning();
        let promise = extract_promises(&doc).unwrap().remove(1);
        let a = load_forge(&forge_dir(), fixture::PACK_NAME).unwrap().pack;

        let mut other = a.pack.clone();
        other.domain = "SYNTHETIC: holdout engineering for invented field instruments".to_owned();
        for w in &mut other.worldviews {
            w.name = format!("SYNTHETIC {}", w.name);
        }
        for sd in &mut other.super_domains {
            for c in &mut sd.categories {
                for item in &mut c.items {
                    for f in &mut item.signature_failures {
                        f.failure = format!("SYNTHETIC: {}", f.failure);
                    }
                }
            }
        }
        let dir = scratch("other-pack");
        let path = dir.join("PACK.json");
        fs::write(&path, other.to_canonical().unwrap()).unwrap();
        let b = load_pack_file(&path).unwrap();

        let (oa, ob) = (DomainOverlay::from_pack(&a), DomainOverlay::from_pack(&b));
        let ca = candidates(&doctrine, &promise, Some(&oa));
        let cb = candidates(&doctrine, &promise, Some(&ob));
        let none = candidates(&doctrine, &promise, None);
        assert_eq!(ca, candidates(&doctrine, &promise, Some(&oa)));
        assert_eq!(cb, candidates(&doctrine, &promise, Some(&ob)));
        assert_ne!(ca, cb);
        assert_ne!(ca, none);
        for (cands, pack) in [(&ca, &a.pack), (&cb, &b.pack)] {
            let failures: Vec<&str> = pack
                .super_domains
                .iter()
                .flat_map(|s| &s.categories)
                .flat_map(|c| &c.items)
                .flat_map(|i| &i.signature_failures)
                .map(|f| f.failure.as_str())
                .collect();
            for c in cands {
                let trace = c
                    .overlay
                    .as_ref()
                    .expect("the overlay shaped the cognition");
                assert_eq!(trace.domain, pack.domain);
                assert!(c.role.contains(&pack.domain), "{}", c.role);
                assert!(
                    c.cannot_assume
                        .iter()
                        .any(|line| failures.iter().any(|f| line.contains(f))),
                    "{:?}",
                    c.cannot_assume
                );
            }
        }
        // Different overlays shift circumstance levels too, not only text.
        let levels = |cs: &[SituatedCognition]| -> Vec<Vec<_>> {
            cs.iter().map(|c| c.circumstance.clone()).collect()
        };
        let mut la = levels(&ca);
        let mut lb = levels(&cb);
        la.sort();
        lb.sort();
        assert_ne!(la, lb);
    }

    fn client_fixture(name: &str) -> PathBuf {
        crate_dir().join("../../fixtures/clients").join(name)
    }

    fn copy_client(name: &str, test: &str) -> PathBuf {
        fn copy(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let destination = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &destination);
                } else {
                    fs::copy(entry.path(), destination).unwrap();
                }
            }
        }
        let destination = scratch(test).join(format!("client-{name}"));
        copy(&client_fixture(name), &destination);
        destination
    }

    fn checkpoint() -> HoldoutCheckpoint {
        HoldoutCheckpoint {
            id: "HOLDOUT-CLIENT-1".to_owned(),
            title: "Public client holdout checkpoint".to_owned(),
            objective: "Exercise the owner's approved promise without revealing parameters."
                .to_owned(),
            acceptance_criterion: "A fresh evaluator returns HOLDOUT_ACCEPTED.".to_owned(),
        }
    }

    /// slice-9 P1: fixed process precedence and sorted paths make detection
    /// deterministic, including when a client carries more than one format.
    /// Fail-without: select the first layout found by HashMap iteration.
    #[test]
    fn detection_is_deterministic() {
        for (name, kind, count) in [
            ("charter", AdapterKind::Charter, 3),
            ("graph", AdapterKind::DispatchGraph, 1),
            ("markdown", AdapterKind::Markdown, 2),
        ] {
            let root = client_fixture(name);
            let expected = detect(&root).unwrap();
            assert_eq!(expected.kind, kind);
            assert_eq!(expected.documents.len(), count);
            assert!(expected.limitations.is_empty());
            assert!(expected.documents.windows(2).all(|w| w[0] < w[1]));
            for _ in 0..32 {
                assert_eq!(detect(&root).unwrap(), expected);
            }
        }

        let mixed = scratch("adapter-mixed-precedence");
        fs::create_dir_all(mixed.join("charter/DONE")).unwrap();
        fs::create_dir_all(mixed.join("plans")).unwrap();
        fs::write(mixed.join("charter/INTENT.md"), b"PROMISE: exact\n").unwrap();
        fs::write(mixed.join("charter/DONE/slice.md"), b"# Pinned\n").unwrap();
        fs::write(mixed.join("plans/dispatch-graph.json"), b"{}\n").unwrap();
        fs::write(mixed.join("README.md"), b"PROMISE: fallback\n").unwrap();
        for _ in 0..64 {
            assert_eq!(detect(&mixed).unwrap().kind, AdapterKind::Charter);
        }
        fs::remove_dir_all(mixed.join("charter")).unwrap();
        for _ in 0..64 {
            assert_eq!(detect(&mixed).unwrap().kind, AdapterKind::DispatchGraph);
        }
        fs::remove_dir_all(mixed.join("plans")).unwrap();
        for _ in 0..64 {
            assert_eq!(detect(&mixed).unwrap().kind, AdapterKind::Markdown);
        }
    }

    /// slice-9 P1: an unrecognized repository gets Markdown behavior and a
    /// stable typed limitation instead of an invented process or a pass.
    /// Fail-without: discard the limitation or return an untyped adapter.
    #[test]
    fn unknown_process_falls_back_with_typed_limitation() {
        let root = scratch("adapter-unknown-process");
        let detection = detect(&root).unwrap();
        assert_eq!(detection.kind, AdapterKind::Markdown);
        assert!(detection.documents.is_empty());
        assert_eq!(detection.limitations.len(), 1);
        assert_eq!(
            detection.limitations[0].code(),
            "unknown_process_markdown_fallback"
        );
        assert_eq!(read_promises(&root).unwrap().adapter, detection);
    }

    /// slice-9 P2: every process yields source-backed byte spans. In
    /// particular double spaces and tabs are not normalized or paraphrased.
    /// Fail-without: normalize whitespace while constructing adapter promises.
    #[test]
    fn promise_extraction_is_verbatim_byte_offset() {
        for name in ["charter", "graph", "markdown"] {
            let evidence = read_promises(&client_fixture(name)).unwrap();
            let promises = evidence
                .documents
                .iter()
                .flat_map(|document| {
                    document
                        .promises
                        .iter()
                        .map(move |promise| (&document.document, promise))
                })
                .collect::<Vec<_>>();
            assert!(!promises.is_empty(), "{name} must expose its promises");
            for (document, promise) in &promises {
                promise.verify_against(document).unwrap();
                let start = usize::try_from(promise.start).unwrap();
                let end = usize::try_from(promise.end).unwrap();
                assert_eq!(promise.text.as_bytes(), &document.bytes[start..end]);
            }
            assert!(
                promises.iter().any(|(_, promise)| {
                    promise.text.contains("  ") || promise.text.contains('\t')
                }),
                "{name} fixture must exercise whitespace preservation"
            );
        }
    }

    /// slice-9 P3: emitted graph checkpoints depend only on QA nodes and the
    /// resulting bytes pass the client repository's own jq validator.
    /// Fail-without: point HOLDOUT depends_on at DEV nodes instead.
    #[test]
    fn graph_writeback_passes_client_validation() {
        let root = copy_client("graph", "adapter-graph-writeback");
        let graph_path = root.join("plans/dispatch-graph.json");
        write_back(
            &root,
            &checkpoint(),
            &WritebackOptions {
                apply: true,
                patch_path: root.join("unused.patch"),
            },
        )
        .unwrap();
        let output = std::process::Command::new("jq")
            .arg("-f")
            .arg("plans/validate-graph.jq")
            .arg("plans/dispatch-graph.json")
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "client validator failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let verdict: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(verdict.get("valid"), Some(&serde_json::Value::Bool(true)));
        assert_eq!(verdict.get("holdout"), Some(&serde_json::Value::from(1)));

        let graph: serde_json::Value =
            serde_json::from_slice(&fs::read(graph_path).unwrap()).unwrap();
        let nodes = graph
            .get("nodes")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        let holdout = nodes
            .iter()
            .find(|node| {
                node.get("id").and_then(serde_json::Value::as_str) == Some("HOLDOUT-CLIENT-1")
            })
            .unwrap();
        let dependencies = holdout
            .get("depends_on")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].as_str(), Some("FEATURE-QA"));
    }

    /// slice-9 P3: only the next-list TEMPLATE receives the holdout proof;
    /// existing pinned DONE bytes remain identical even during apply mode.
    /// Fail-without: select an existing DONE list as the charter target.
    #[test]
    fn charter_writeback_never_touches_pinned_lists() {
        let root = copy_client("charter", "adapter-charter-writeback");
        let pinned = root.join("charter/DONE/slice-1.md");
        let before = fs::read(&pinned).unwrap();
        let proposed = WritebackOptions {
            apply: false,
            patch_path: pinned.clone(),
        };
        assert!(matches!(
            write_back(&root, &checkpoint(), &proposed),
            Err(AdapterError::PinnedDoneList(_))
        ));
        assert_eq!(fs::read(&pinned).unwrap(), before);

        write_back(
            &root,
            &checkpoint(),
            &WritebackOptions {
                apply: true,
                patch_path: root.join("unused.patch"),
            },
        )
        .unwrap();
        assert_eq!(fs::read(pinned).unwrap(), before);
        let template = fs::read_to_string(root.join("charter/DONE/TEMPLATE.md")).unwrap();
        assert!(template.contains(
            "- Holdout proof HOLDOUT-CLIENT-1: Exercise the owner's approved promise without revealing parameters."
        ));
    }

    /// slice-9 P4: default options write only a patch; explicit apply changes
    /// the native planning document and causes no proposal to be emitted.
    /// Fail-without: apply the write-back in place under default options.
    #[test]
    fn writeback_is_proposal_only_by_default() {
        let root = copy_client("graph", "adapter-proposal-default");
        let target = root.join("plans/dispatch-graph.json");
        let before = fs::read(&target).unwrap();
        let options = WritebackOptions {
            patch_path: root.join(".holden/proposal.patch"),
            ..WritebackOptions::default()
        };
        let outcome = write_back(&root, &checkpoint(), &options).unwrap();
        assert!(!outcome.applied);
        assert_eq!(outcome.patch, Some(PathBuf::from(".holden/proposal.patch")));
        assert_eq!(fs::read(&target).unwrap(), before);
        let patch = fs::read_to_string(root.join(".holden/proposal.patch")).unwrap();
        assert!(patch.contains("--- a/plans/dispatch-graph.json"));
        assert!(patch.contains("HOLDOUT-CLIENT-1"));

        let applied = write_back(
            &root,
            &checkpoint(),
            &WritebackOptions {
                apply: true,
                ..options
            },
        )
        .unwrap();
        assert!(applied.applied);
        assert_eq!(applied.patch, None);
        assert_ne!(fs::read(target).unwrap(), before);
        assert!(root.join(".holden/proposal.patch").is_file());
    }

    /// slice-18b: a charter's narrative remains context; its numbered DONE
    /// proofs are the source-backed owner candidates.
    /// Fail-without: restore paragraph extraction on charter documents.
    #[test]
    fn prose_charter_yields_verbatim_candidates() {
        let evidence = read_promises(&client_fixture("prose-charter")).unwrap();
        assert_eq!(evidence.adapter.kind, AdapterKind::Charter);
        assert_eq!(evidence.discovery, DiscoveryKind::Explicit);
        let intent = evidence
            .documents
            .iter()
            .find(|document| document.document.name == "charter/INTENT.md")
            .unwrap();
        assert!(intent.promises.is_empty());
        assert_eq!(evidence.spans().len(), 1);
        assert!(evidence.spans().iter().any(|(document, promise)| {
            document.name == "charter/DONE/slice-1.md"
                && promise.text == "P1 source proof: the tool keeps  every owner-approved planning quote available to a successor."
                && promise.start > 0
                && promise.end > promise.start
        }));
        for (document, promise) in evidence.spans() {
            promise.verify_against(document).unwrap();
        }
    }

    /// slice-13 P2: ordinary Markdown prose yields candidates and the public
    /// detection result names the Markdown planning process.
    /// Fail-without: use marker-only extraction instead of the prose fallback.
    #[test]
    fn prose_markdown_yields_candidates_and_names_markdown() {
        let evidence = read_promises(&client_fixture("prose-markdown")).unwrap();
        assert_eq!(evidence.adapter.kind, AdapterKind::Markdown);
        assert_eq!(evidence.adapter.kind.as_str(), "markdown");
        assert_eq!(evidence.discovery, DiscoveryKind::Prose);
        assert_eq!(evidence.documents.len(), 2);
        assert!(evidence.spans().iter().any(|(document, promise)| {
            document.name == "docs/PRD.md"
                && promise.text
                    == "An agent can trace each approved requirement to its exact source bytes."
        }));
    }

    /// slice-13 P5: a proposal is its exact source byte span, including
    /// repeated spaces and an interior newline; no trim or normalization is
    /// hidden by comparing only display text.
    /// Fail-without: trim or normalize text while constructing a proposal.
    #[test]
    fn proposals_are_byte_exact_substrings() {
        let mut checked = 0usize;
        for fixture in ["prose-charter", "prose-markdown"] {
            let evidence = read_promises(&client_fixture(fixture)).unwrap();
            assert_eq!(
                evidence.discovery,
                if fixture == "prose-charter" {
                    DiscoveryKind::Explicit
                } else {
                    DiscoveryKind::Prose
                }
            );
            for (document, promise) in evidence.spans() {
                let start = usize::try_from(promise.start).unwrap();
                let end = usize::try_from(promise.end).unwrap();
                assert_eq!(promise.text.as_bytes(), &document.bytes[start..end]);
                assert_eq!(promise.end - promise.start, promise.text.len() as u64);
                promise.verify_against(document).unwrap();
                checked += 1;
            }
        }
        assert!(checked >= 3);
        let evidence = read_promises(&client_fixture("prose-charter")).unwrap();
        assert!(
            evidence
                .spans()
                .iter()
                .any(|(_, promise)| promise.text.contains("  "))
        );
        let markdown = read_promises(&client_fixture("prose-markdown")).unwrap();
        assert!(
            markdown
                .spans()
                .iter()
                .any(|(_, promise)| promise.text.ends_with("  ")),
            "trailing source spaces must survive proposal construction"
        );
    }
}
