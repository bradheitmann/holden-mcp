//! holden-canon: typed canon pins, byte-for-byte canon verification, and the
//! lens corpus model.
//!
//! - [`pin`]: the `holden.pin/1` format (`canon/CANON-PIN.txt`, lens manifests).
//! - [`verify`](mod@verify): recompute SHA-256 for every pinned file and refuse any
//!   differing, missing or extra file, naming it. An absent root is the typed
//!   limitation `canon_absent`, never a pass.
//! - [`hosted`]: the recorded S-CANON-MCP `check_canon_lock` response.
//! - [`lens`]: concepts and situated cognitions with content-hash ids.
//! - [`import`]: S-LENS registries to the typed corpus, with declared deltas.
//! - [`doctrine`]: the typed doctrine (definition, roots, constituencies,
//!   circumstance axes, generator rules), every claim cited into the canon.

pub mod digest;
pub mod doctrine;
pub mod hosted;
pub mod import;
pub mod lens;
pub mod pin;
pub mod verify;

pub use digest::Sha256Digest;
pub use doctrine::{
    AxisLevel, CircumstanceAxis, Citation, Constituency, Doctrine, PlanPart, Root, RootId,
};
pub use hosted::HostedLockRecord;
pub use lens::{Concept, ConceptKind, LensCorpus, SituatedCognition};
pub use pin::{Pin, PinnedPath};
pub use verify::{CanonError, Finding, VerifyReport, verify};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::*;

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn fixture(rel: &str) -> PathBuf {
        crate_dir().join("fixtures").join(rel)
    }

    fn read_pin(path: &Path) -> Pin {
        Pin::parse(&fs::read_to_string(path).unwrap()).unwrap()
    }

    /// A fresh, empty scratch directory unique to this test and process.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("holden-canon-test-{name}-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn copy_tree(from: &Path, to: &Path) {
        fs::create_dir_all(to).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let dst = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &dst);
            } else {
                fs::copy(entry.path(), dst).unwrap();
            }
        }
    }

    fn synthetic_copy(name: &str) -> (PathBuf, Pin) {
        let root = scratch(name).join("root");
        copy_tree(&fixture("canon-synthetic/root"), &root);
        (root, read_pin(&fixture("canon-synthetic/CANON-PIN.txt")))
    }

    #[test]
    fn committed_synthetic_canon_fixture_verifies() {
        let pin = read_pin(&fixture("canon-synthetic/CANON-PIN.txt"));
        let report = verify(&fixture("canon-synthetic/root"), &pin).unwrap();
        assert_eq!(report.file_count, 3);
        assert_eq!(report.canon_version, "synthetic-fixture-1");
        assert_eq!(report.digest, pin.listing_digest());
    }

    #[test]
    fn verify_refuses_modified_byte() {
        let (root, pin) = synthetic_copy("modified-byte");
        verify(&root, &pin).unwrap();
        let target = root.join("taxonomy/codes.md");
        let mut bytes = fs::read(&target).unwrap();
        bytes[0] ^= 0x01;
        fs::write(&target, bytes).unwrap();
        match verify(&root, &pin) {
            Err(CanonError::Refused(findings)) => {
                assert_eq!(findings.len(), 1);
                assert!(matches!(&findings[0], Finding::Differs { .. }));
                assert_eq!(findings[0].file(), "taxonomy/codes.md");
            }
            other => panic!("expected a refusal naming the file, got {other:?}"),
        }
    }

    #[test]
    fn verify_refuses_missing_file() {
        let (root, pin) = synthetic_copy("missing-file");
        fs::remove_file(root.join("schema.yaml")).unwrap();
        match verify(&root, &pin) {
            Err(CanonError::Refused(findings)) => {
                assert_eq!(findings.len(), 1);
                assert!(matches!(&findings[0], Finding::Missing { .. }));
                assert_eq!(findings[0].file(), "schema.yaml");
            }
            other => panic!("expected a missing-file refusal, got {other:?}"),
        }
    }

    #[test]
    fn verify_refuses_extra_file() {
        let (root, pin) = synthetic_copy("extra-file");
        fs::write(root.join("taxonomy/unpinned.md"), b"extra").unwrap();
        match verify(&root, &pin) {
            Err(CanonError::Refused(findings)) => {
                assert_eq!(findings.len(), 1);
                assert!(matches!(&findings[0], Finding::Extra { .. }));
                assert_eq!(findings[0].file(), "taxonomy/unpinned.md");
            }
            other => panic!("expected an extra-file refusal, got {other:?}"),
        }
    }

    #[test]
    fn absent_root_is_limitation_not_pass() {
        let pin = read_pin(&fixture("canon-synthetic/CANON-PIN.txt"));
        let absent = scratch("absent-root").join("does-not-exist");
        let result = verify(&absent, &pin);
        let err = result.expect_err("an absent root must never verify");
        assert!(err.is_limitation());
        assert_eq!(err.code(), "canon_absent");
        assert!(matches!(err, CanonError::CanonAbsent { .. }));
    }

    /// The committed S-DOCTRINE canon (`canon/holden/`) is byte-identical to
    /// the hosted pin `canon/CANON-PIN.txt` (slice-1b P3).
    #[test]
    fn committed_canon_matches_pin() {
        let repo = crate_dir().join("../..");
        let pin = read_pin(&repo.join("canon/CANON-PIN.txt"));
        assert_eq!(pin.canon_version, "skill-v2.3.0 adapter-v0.2.0");
        let report = verify(&repo.join("canon/holden"), &pin).unwrap();
        assert_eq!(report.file_count, 4);
        assert_eq!(report.digest, pin.listing_digest());
    }

    #[test]
    fn pin_matches_recorded_hosted_pin() {
        let repo = crate_dir().join("../..");
        let pin = read_pin(&repo.join("canon/CANON-PIN.txt"));
        let recorded =
            fs::read_to_string(repo.join("docs/evidence/canon-mcp-pin-2026-11-04.txt")).unwrap();
        let hosted = HostedLockRecord::parse(&recorded).unwrap();
        assert_eq!(hosted.canon_version, "skill-v2.3.0 adapter-v0.2.0");
        assert_eq!(hosted.files.len(), 4);
        assert_eq!(pin.files.len(), 4);
        assert_eq!(hosted.differences(&pin), Vec::<String>::new());
        assert_eq!(pin.files, hosted.files);
        assert_eq!(pin.validator_hash, Some(hosted.validator_hash));
    }

    #[test]
    fn lens_ids_are_content_hashes() {
        let dir = fixture("lens-synthetic/corpus");
        let concepts: Vec<Concept> = lens::read_json(&dir.join(lens::files::CONCEPTS)).unwrap();
        let cognitions: Vec<SituatedCognition> =
            lens::read_json(&dir.join(lens::files::COGNITIONS)).unwrap();
        assert!(!concepts.is_empty());
        assert!(!cognitions.is_empty());
        for c in &concepts {
            assert_eq!(c.id, c.computed_id(), "concept '{}'", c.name);
        }
        for s in &cognitions {
            assert_eq!(s.id, s.computed_id(), "cognition '{}'", s.level);
        }
        let corpus = LensCorpus::load(&dir).unwrap();
        corpus.validate().unwrap();
    }

    #[test]
    fn lens_fixture_matches_its_manifest() {
        let pin = read_pin(&fixture("lens-synthetic/MANIFEST.sha256"));
        verify(&fixture("lens-synthetic/corpus"), &pin).unwrap();
    }

    #[test]
    fn import_reproduces_committed_lens_fixture() {
        let out = scratch("import").join("lens");
        let outcome = import::import_lens(&fixture("lens-source-synthetic")).unwrap();
        import::write_corpus(&out, &outcome).unwrap();
        let committed = read_pin(&fixture("lens-synthetic/MANIFEST.sha256"));
        let written = read_pin(&out.join("MANIFEST.sha256"));
        assert_eq!(written, committed);
        verify(&out.join("corpus"), &committed).unwrap();
    }

    #[test]
    fn import_refuses_non_empty_destination() {
        let out = scratch("import-non-empty");
        fs::write(out.join("keep.txt"), b"x").unwrap();
        let outcome = import::import_lens(&fixture("lens-source-synthetic")).unwrap();
        assert!(import::write_corpus(&out, &outcome).is_err());
        assert_eq!(fs::read(out.join("keep.txt")).unwrap(), b"x");
    }

    fn repo_canon() -> PathBuf {
        crate_dir().join("../../canon/holden")
    }

    /// slice-3 P1: every citation of the doctrine (definition, roots,
    /// constituencies, derivations) resolves to existing section bytes in the
    /// committed canon, and every root cites at least one section.
    #[test]
    fn every_root_citation_resolves() {
        let d = Doctrine::holden();
        let ids: Vec<RootId> = d.roots.iter().map(|r| r.id).collect();
        assert_eq!(ids, RootId::ALL.to_vec());
        for r in &d.roots {
            assert!(
                !r.citations.is_empty(),
                "root {} cites nothing",
                r.id.as_str()
            );
            for c in &r.citations {
                assert!(
                    c.resolve(&repo_canon()).is_ok(),
                    "root {}: {:?}",
                    r.id.as_str(),
                    c.resolve(&repo_canon())
                );
            }
        }
        match d.verify(&repo_canon()) {
            Ok(n) => assert_eq!(n, d.citations().len()),
            Err(failures) => {
                let lines: Vec<String> = failures
                    .iter()
                    .map(|(at, e)| format!("{at}: {e}"))
                    .collect();
                panic!("unresolved citations:\n{}", lines.join("\n"));
            }
        }
    }

    /// slice-3 P1: every root has a generator rule that yields at least one
    /// constituency and one axis, and together the rules span every
    /// constituency and every axis.
    #[test]
    fn every_root_generates_something() {
        let d = Doctrine::holden();
        let mut constituencies = std::collections::BTreeSet::new();
        let mut axes = std::collections::BTreeSet::new();
        for id in RootId::ALL {
            let g = d.generator(id).unwrap();
            assert!(
                !g.constituencies.is_empty(),
                "{} yields no constituency",
                id.as_str()
            );
            assert!(!g.axes.is_empty(), "{} yields no axis", id.as_str());
            constituencies.extend(g.constituencies.iter().copied());
            axes.extend(g.axes.iter().copied());
        }
        assert_eq!(d.generators.len(), RootId::ALL.len());
        assert_eq!(
            constituencies.into_iter().collect::<Vec<_>>(),
            Constituency::ALL.to_vec()
        );
        assert_eq!(
            axes.into_iter().collect::<Vec<_>>(),
            CircumstanceAxis::ALL.to_vec()
        );
        for a in &d.axes {
            assert!(a.levels.len() >= 3, "{} has too few levels", a.id.as_str());
            assert!(a.levels.iter().all(|l| l.axis() == a.id));
        }
    }

    /// A changed byte inside a cited section is refused, naming the section.
    #[test]
    fn doctrine_refuses_changed_canon_section() {
        let dir = scratch("doctrine-changed");
        copy_tree(&repo_canon(), &dir);
        let file = dir.join(doctrine::SPEC);
        let mut bytes = fs::read(&file).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        let at = text.find("### 1.6 The Nothing Doctrine").unwrap() + 40;
        bytes[at] ^= 0x01;
        fs::write(&file, bytes).unwrap();
        let failures = Doctrine::holden().verify(&dir).unwrap_err();
        assert!(!failures.is_empty());
        assert!(failures.iter().all(|(_, e)| matches!(
            e,
            doctrine::CitationError::HashDiffers { heading, .. } if heading == "### 1.6 The Nothing Doctrine"
        )), "{failures:?}");
    }

    #[test]
    fn section_bytes_stop_at_same_or_higher_heading_outside_fences() {
        let text = b"# T\n## A\na\n```\n## not a heading\n```\n### A.1\nb\n## B\nc\n";
        let a = doctrine::section_bytes(text, "## A").unwrap();
        assert_eq!(a, b"## A\na\n```\n## not a heading\n```\n### A.1\nb\n");
        let a1 = doctrine::section_bytes(text, "### A.1").unwrap();
        assert_eq!(a1, b"### A.1\nb\n");
        assert_eq!(doctrine::section_bytes(text, "## B").unwrap(), b"## B\nc\n");
        assert!(doctrine::section_bytes(text, "## not a heading").is_err());
        assert!(doctrine::section_bytes(b"## X\n## X\n", "## X").is_err());
    }

    #[test]
    fn pin_parse_refuses_uppercase_or_short_digest() {
        let base = "format: holden.pin/1\ncanon_version: v\n";
        let upper = format!("{base}{}  a.md\n", "A".repeat(64));
        let short = format!("{base}{}  a.md\n", "a".repeat(63));
        assert!(Pin::parse(&upper).is_err());
        assert!(Pin::parse(&short).is_err());
        assert!(Pin::parse(&format!("{base}{}  a.md\n", "a".repeat(64))).is_ok());
    }

    #[test]
    fn pin_parse_refuses_escaping_paths() {
        let base = "format: holden.pin/1\ncanon_version: v\n";
        for bad in ["/abs.md", "../up.md", "a/./b.md", "a//b.md"] {
            let text = format!("{base}{}  {bad}\n", "a".repeat(64));
            assert!(Pin::parse(&text).is_err(), "{bad} must be refused");
        }
    }
}
