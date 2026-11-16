//! The module's proofs. Each test names its fail-without mutant.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constitution::{self, Constitution};
use crate::export::{self, Denylist, ExportError};
use crate::json::{self, Value};
use crate::pack::{self, Pack, Role};
use crate::{DEFAULT_PACK, constitution_path, module_root};

/// A fresh path under the system temp directory, unique per call even when
/// tests run in parallel.
fn scratch(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!(
        "holden-forge-test-{name}-{}-{n}-{nanos}",
        std::process::id()
    ))
}

fn pack_text() -> String {
    fs::read_to_string(module_root().join(DEFAULT_PACK)).unwrap()
}

fn loaded() -> pack::LoadedPack {
    pack::load(&pack_text()).unwrap()
}

/// A sentinel identifier, assembled at run time so no module file holds it.
fn sentinel() -> String {
    ["zz-private", "-sentinel"].concat()
}

/// A denylist for tests: a sentinel no module file may contain.
fn test_denylist() -> Denylist {
    Denylist::parse(&format!("# test denylist\n{}\n", sentinel()))
}

/// Copy the module's file set into a fresh directory outside any repository.
fn copy_module() -> PathBuf {
    let src = module_root();
    let dir = scratch("module");
    for f in export::module_files(&src).unwrap() {
        let to = dir.join(&f);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::copy(src.join(&f), &to).unwrap();
    }
    fs::copy(src.join(export::SUMS), dir.join(export::SUMS)).unwrap();
    dir
}

fn append(path: &Path, extra: &str) {
    let mut text = fs::read_to_string(path).unwrap();
    text.push_str(extra);
    fs::write(path, text).unwrap();
}

/// P3: the holdout-engineering pack validates against the typed schema, and
/// the schema refuses an unknown member, a missing member, an empty
/// depth-contract list, a role cut naming a missing super-domain and a
/// non-canonical layout, each named by its JSON pointer.
/// Fail-without mutant: make `validate` skip the empty-list check; the
/// empty `anti_signals` case is accepted and this test fails.
#[test]
fn pack_validates() {
    let l = loaded();
    assert_eq!(l.pack.pack_name, "holdout-engineering");
    assert_eq!(l.pack.super_domains.len(), 5);
    assert_eq!(l.pack.category_count(), 10);
    assert_eq!(l.pack.item_count(), 16);
    for role in Role::ALL {
        assert!(l.pack.focus(role).is_some(), "{role:?}");
    }

    let text = pack_text();
    let refuse = |bad: &str, path: &str| {
        let errs = pack::load(bad).expect_err("must refuse");
        assert!(
            errs.iter().any(|e| e.path == path),
            "expected an error at {path}, got {errs:?}"
        );
    };
    refuse(
        &text.replacen("\"domain\": ", "\"extra\": \"x\",\n  \"domain\": ", 1),
        "/extra",
    );
    let mut v = json::parse(&text).unwrap();
    if let Value::Object(m) = &mut v {
        m.retain(|(k, _)| k != "module_laws");
    }
    refuse(&json::to_canonical(&v), "/module_laws");

    let mut p: Pack = l.pack.clone();
    p.super_domains[0].categories[0].items[0]
        .anti_signals
        .clear();
    refuse(
        &p.to_canonical(),
        "/super_domains/0/categories/0/items/0/anti_signals",
    );
    let mut p: Pack = l.pack.clone();
    p.role_focus[1].super_domains[0] = "Z".into();
    refuse(&p.to_canonical(), "/role_focus/1/super_domains/0");
    refuse(&text.replacen("{\n  \"schema\"", "{ \"schema\"", 1), "");
}

/// The section names of the derivation map in `methodology/LW-AGENT-FORGE.md` §2.
fn methodology_sections() -> Vec<String> {
    let text = fs::read_to_string(module_root().join("methodology/LW-AGENT-FORGE.md")).unwrap();
    let start = text.find("## 2. Derivation map").unwrap();
    let end = start + text[start..].find("**Citation rule:**").unwrap();
    text[start..end]
        .lines()
        .filter(|l| l.starts_with("| ") && !l.starts_with("| Pack source"))
        .filter_map(|l| {
            // The bold name, ignoring any note after it, e.g. "(per super-domain)".
            let cell = l.split('|').nth(2)?.trim();
            let (name, _) = cell.strip_prefix("**")?.split_once("**")?;
            Some(name.to_owned())
        })
        .collect()
}

fn arr<'a>(v: &'a Value, path: &str) -> &'a [Value] {
    match v.pointer(path) {
        Some(Value::Array(a)) => a,
        _ => &[],
    }
}

/// The trace set each section must carry for `role`, computed from the raw
/// pack JSON and the derivation map, independently of `constitution::derive`.
fn expected_traces(v: &Value, role: &str) -> Vec<BTreeSet<String>> {
    let mut s: Vec<BTreeSet<String>> = vec![BTreeSet::new(); 9];
    let ri = arr(v, "/role_focus")
        .iter()
        .position(|r| r.get("role").and_then(Value::as_str) == Some(role))
        .unwrap();
    let rf = format!("/role_focus/{ri}");
    let codes: Vec<&str> = arr(v, &format!("{rf}/super_domains"))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let wids: Vec<&str> = arr(v, &format!("{rf}/worldviews"))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let mut add = |n: usize, p: String| {
        s[n].insert(p);
    };
    add(0, format!("{rf}/identity"));
    for k in ["/pack_name", "/domain", "/ratification_status"] {
        add(0, k.to_owned());
    }
    for i in 0..arr(v, "/priority_order").len() {
        add(0, format!("/priority_order/{i}"));
    }
    for (wi, w) in arr(v, "/worldviews").iter().enumerate() {
        if wids.contains(&w.get("id").and_then(Value::as_str).unwrap()) {
            add(2, format!("/worldviews/{wi}/name"));
            add(2, format!("/worldviews/{wi}/applies_when"));
        }
    }
    for (si, sd) in arr(v, "/super_domains").iter().enumerate() {
        let sb = format!("/super_domains/{si}");
        add(1, format!("{sb}/name"));
        let mine = codes.contains(&sd.get("code").and_then(Value::as_str).unwrap());
        for ci in 0..arr(v, &format!("{sb}/categories")).len() {
            let cb = format!("{sb}/categories/{ci}");
            add(1, format!("{cb}/name"));
            add(1, format!("{cb}/seam"));
            if !mine {
                continue;
            }
            for ii in 0..arr(v, &format!("{cb}/items")).len() {
                let ib = format!("{cb}/items/{ii}");
                for k in 0..arr(v, &format!("{ib}/synthesis_instructions")).len() {
                    add(3, format!("{ib}/synthesis_instructions/{k}"));
                }
                for k in 0..arr(v, &format!("{ib}/internal_evaluation_criteria")).len() {
                    for f in ["question", "pass_condition", "checklist"] {
                        add(4, format!("{ib}/internal_evaluation_criteria/{k}/{f}"));
                    }
                }
                for list in ["applicability_signals", "anti_signals"] {
                    for k in 0..arr(v, &format!("{ib}/{list}")).len() {
                        add(5, format!("{ib}/{list}/{k}"));
                    }
                }
                for k in 0..arr(v, &format!("{ib}/signature_failures")).len() {
                    for f in ["failure", "guard"] {
                        add(6, format!("{ib}/signature_failures/{k}/{f}"));
                    }
                }
            }
        }
    }
    for i in 0..arr(v, "/cross_cutting_postures").len() {
        add(7, format!("/cross_cutting_postures/{i}/name"));
        add(7, format!("/cross_cutting_postures/{i}/posture"));
    }
    for i in 0..arr(v, "/module_laws").len() {
        add(8, format!("/module_laws/{i}/law"));
    }
    s
}

/// The `CC.II` citation of the item a trace points into, if any.
fn citation_of(v: &Value, trace: &str) -> Option<String> {
    let parts: Vec<&str> = trace.split('/').collect();
    // ["", "super_domains", si, "categories", ci, "items", ii, ...]
    if parts.len() < 7 || parts[1] != "super_domains" || parts[5] != "items" {
        return None;
    }
    let cat = v.pointer(&parts[..5].join("/"))?;
    let item = v.pointer(&parts[..7].join("/"))?;
    Some(format!(
        "{}.{}",
        cat.get("code")?.as_str()?,
        item.get("item_code")?.as_str()?
    ))
}

/// P3: for every role, the constitution has exactly the derivation map's
/// sections in its order; every entry's text equals the pack value its
/// trace points to; item-derived entries cite the item they come from; and
/// each section carries exactly the pack fields the map feeds it (nothing
/// dropped, nothing extra).
/// Fail-without mutant: drop one pack field from the derivation (for
/// example stop emitting `anti_signals` into TASK ROUTING); the trace set
/// of that section no longer matches and this test fails.
#[test]
fn constitution_sections_trace_to_pack() {
    let text = pack_text();
    let v = json::parse(&text).unwrap();
    let l = loaded();
    let titles = methodology_sections();
    assert_eq!(
        titles.len(),
        9,
        "the methodology's derivation map has nine sections"
    );
    for role in Role::ALL {
        let c = constitution::derive(&l, role).unwrap();
        assert_eq!(c.pack_digest, pack::pack_digest(text.as_bytes()));
        let got: Vec<&str> = c.sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            got, titles,
            "{role:?}: sections must follow the derivation map"
        );
        let expected = expected_traces(&v, role.as_str());
        for (n, s) in c.sections.iter().enumerate() {
            assert!(!s.entries.is_empty(), "{role:?}: {} is empty", s.title);
            let traces: BTreeSet<String> = s.entries.iter().map(|e| e.trace.clone()).collect();
            assert_eq!(
                traces.len(),
                s.entries.len(),
                "{role:?}: {} repeats a trace",
                s.title
            );
            assert_eq!(
                traces, expected[n],
                "{role:?}: {} does not carry exactly its pack fields",
                s.title
            );
            for e in &s.entries {
                let value = v.pointer(&e.trace).and_then(Value::as_str);
                assert_eq!(value, Some(e.text.as_str()), "{role:?}: {}", e.trace);
                if let Some(cite) = citation_of(&v, &e.trace) {
                    let tag_cite = e.tag.split(':').next().unwrap();
                    if s.title == "SELF-QC PROTOCOL" {
                        let item_id = cite.replace('.', "");
                        assert!(tag_cite.starts_with(&format!("{item_id}-C")), "{}", e.tag);
                    } else {
                        assert_eq!(tag_cite, cite, "{role:?}: {}", e.trace);
                    }
                }
            }
        }
    }
}

/// P3: output is deterministic. Two derivations give the same bytes, the
/// committed constitutions under `agents/` equal a fresh emission (they are
/// not stale and were not hand-edited), and two exports of the module give
/// identical `SHA256SUMS`.
/// Fail-without mutant: make `render` include anything that varies per run
/// (a timestamp, a hash-map order); the byte comparison fails.
#[test]
fn deterministic_output() {
    let l1 = loaded();
    let l2 = loaded();
    assert_eq!(l1.digest, l2.digest);
    for role in Role::ALL {
        let a = constitution::render(&constitution::derive(&l1, role).unwrap());
        let b = constitution::render(&constitution::derive(&l2, role).unwrap());
        assert_eq!(a, b);
        let committed =
            fs::read_to_string(module_root().join(constitution_path(&l1.pack.pack_name, role)))
                .unwrap();
        assert_eq!(
            committed, a,
            "{role:?}: committed constitution is stale; re-emit it"
        );
    }
    let (d1, d2) = (scratch("det-a"), scratch("det-b"));
    export::export(&module_root(), &d1, &test_denylist()).unwrap();
    export::export(&module_root(), &d2, &test_denylist()).unwrap();
    assert_eq!(
        fs::read(d1.join(export::SUMS)).unwrap(),
        fs::read(d2.join(export::SUMS)).unwrap()
    );
    fs::remove_dir_all(&d1).unwrap();
    fs::remove_dir_all(&d2).unwrap();
}

/// P3: the typed reader loses nothing. The pack file reads to a typed pack
/// that writes back to the same bytes and reads back equal; each emitted
/// constitution parses back to the same IR and renders to the same bytes,
/// and the reader refuses a constitution with one changed entry layout.
/// Fail-without mutant: drop the `checklist` member from the pack writer
/// (or the trace from the constitution renderer); the bytes differ.
#[test]
fn roundtrip_lossless() {
    let text = pack_text();
    let l = loaded();
    assert_eq!(l.pack.to_canonical(), text);
    let again = Pack::from_value(&json::parse(&l.pack.to_canonical()).unwrap()).unwrap();
    assert_eq!(again, l.pack);
    for role in Role::ALL {
        let c = constitution::derive(&l, role).unwrap();
        let rendered = constitution::render(&c);
        let back: Constitution = constitution::parse(&rendered).unwrap();
        assert_eq!(back, c);
        assert_eq!(constitution::render(&back), rendered);
        let broken = rendered.replacen(" [trace: /pack_name]", " [trace:/pack_name]", 1);
        assert!(constitution::parse(&broken).is_err());
    }
}

/// P2: an export refuses, naming the file, when an exported methodology
/// file holds an absolute home path, and writes nothing.
/// Fail-without mutant: remove the absolute-path check from `export::scan`;
/// the export succeeds and this test fails.
#[test]
fn export_refuses_absolute_path() {
    let src = copy_module();
    let home = ["/", "Users", "/", "someone", "/notes.md"].concat();
    append(
        &src.join("methodology/LW-AGENT-FORGE.md"),
        &format!("\nSee {home}.\n"),
    );
    let dest = scratch("abs-dest");
    match export::export(&src, &dest, &test_denylist()) {
        Err(ExportError::Refused(f)) => {
            assert!(
                f.iter()
                    .any(|x| x.starts_with("methodology/LW-AGENT-FORGE.md:")
                        && x.ends_with("absolute path")),
                "{f:?}"
            );
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert!(!dest.exists(), "a refused export writes nothing");
    fs::remove_dir_all(&src).unwrap();
}

/// P2: an export refuses a private identifier from the denylist, naming the
/// file and the entry number but never the identifier.
/// Fail-without mutant: skip the denylist loop in `export::scan`.
#[test]
fn export_refuses_private_identifier() {
    let src = copy_module();
    append(
        &src.join("README.md"),
        &format!("\nWritten for {}.\n", sentinel().to_uppercase()),
    );
    let dest = scratch("deny-dest");
    match export::export(&src, &dest, &test_denylist()) {
        Err(ExportError::Refused(f)) => {
            assert!(
                f.iter().any(|x| x.starts_with("README.md:")
                    && x.contains("denylist entry #2")
                    && !x.to_lowercase().contains("sentinel")),
                "{f:?}"
            );
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    fs::remove_dir_all(&src).unwrap();
}

/// P2: an export refuses a reference to the parent repository.
/// Fail-without mutant: empty `parent_references`.
#[test]
fn export_refuses_parent_reference() {
    let src = copy_module();
    let parent = ["../../", "modules", "/", "holden-forge"].concat();
    append(
        &src.join("UPSTREAM.md"),
        &format!("\nBuilt from {parent}.\n"),
    );
    let dest = scratch("parent-dest");
    match export::export(&src, &dest, &test_denylist()) {
        Err(ExportError::Refused(f)) => {
            assert!(
                f.iter().any(|x| x.starts_with("UPSTREAM.md:")
                    && x.ends_with("reference to the parent repository")),
                "{f:?}"
            );
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    fs::remove_dir_all(&src).unwrap();
}

fn nested_target() -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR").filter(|v| !v.is_empty()) {
        Some(t) => {
            let mut s = t;
            s.push("-export");
            PathBuf::from(s)
        }
        None => scratch("export-target"),
    }
}

fn run_cargo(dir: &Path, target: &Path, args: &[&str]) -> std::process::Output {
    let out = Command::new("cargo")
        .current_dir(dir)
        .args(args)
        .env("CARGO_TARGET_DIR", target)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("FORGE_DENYLIST")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cargo {} failed in the export:\n{}\n{}",
        args.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// P2: the export is a standalone module. It lands in a fresh directory
/// outside the repository with a regenerated, verifying `SHA256SUMS`; there
/// it builds, passes its own tests (all but this one, which would recurse)
/// and verifies itself with no network; and `cargo metadata` shows no
/// package or manifest outside the export.
/// Fail-without mutant: add a path dependency outside the module to its
/// `Cargo.toml`; the export's metadata check (and its build) fails.
#[test]
fn export_builds_standalone() {
    let dest = scratch("standalone");
    let report = export::export(&module_root(), &dest, &test_denylist()).unwrap();
    assert_eq!(export::verify_sums(&dest).unwrap(), report.files);
    assert!(export::check_license(&dest).is_empty());
    let target = nested_target();
    run_cargo(&dest, &target, &["build", "--offline", "--locked"]);
    run_cargo(
        &dest,
        &target,
        &[
            "test",
            "--offline",
            "--locked",
            "--",
            "--skip",
            "export_builds_standalone",
        ],
    );
    let verify = run_cargo(
        &dest,
        &target,
        &["run", "--offline", "--locked", "--quiet", "--", "verify"],
    );
    assert!(String::from_utf8_lossy(&verify.stdout).contains("verified"));
    let meta = run_cargo(
        &dest,
        &target,
        &[
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--no-deps",
        ],
    );
    let v = json::parse(&String::from_utf8_lossy(&meta.stdout)).unwrap();
    let canon = fs::canonicalize(&dest).unwrap();
    let inside = |p: &str| fs::canonicalize(p).is_ok_and(|c| c.starts_with(&canon));
    assert!(inside(
        v.get("workspace_root").and_then(Value::as_str).unwrap()
    ));
    for p in arr(&v, "/packages") {
        assert!(inside(
            p.get("manifest_path").and_then(Value::as_str).unwrap()
        ));
        for d in match p.get("dependencies") {
            Some(Value::Array(a)) => a.as_slice(),
            _ => &[],
        } {
            if let Some(path) = d.get("path").and_then(Value::as_str) {
                assert!(inside(path), "path dependency outside the export");
            }
        }
    }
    fs::remove_dir_all(&dest).unwrap();
}

/// P4: `SHA256SUMS` covers every module file with a matching hash, and the
/// verify step refuses, naming the file, when one methodology file changes
/// without `SHA256SUMS` being updated.
/// Fail-without mutant: make `verify_sums` skip the hash comparison; the
/// changed-file case passes and this test fails.
#[test]
fn sums_cover_every_file() {
    let root = module_root();
    let files = export::module_files(&root).unwrap();
    assert_eq!(export::verify_sums(&root), Ok(files.len()));
    let listed: Vec<String> = fs::read_to_string(root.join(export::SUMS))
        .unwrap()
        .lines()
        .map(|l| l.split_once("  ").unwrap().1.to_owned())
        .collect();
    assert_eq!(listed, files, "every module file, in order");
    for must in ["LICENSE", "NOTICE", "VERSION", "UPSTREAM.md", "Cargo.lock"] {
        assert!(files.iter().any(|f| f == must), "{must}");
    }

    let copy = copy_module();
    append(&copy.join("methodology/BOOT-CONTRACT.md"), "\nchanged\n");
    let findings = export::verify_sums(&copy).expect_err("a changed file must fail verify");
    assert_eq!(
        findings,
        vec!["methodology/BOOT-CONTRACT.md: sha256 does not match SHA256SUMS".to_owned()]
    );
    fs::write(copy.join("extra.txt"), "x").unwrap();
    let findings = export::verify_sums(&copy).expect_err("an unlisted file must fail verify");
    assert!(findings.contains(&"extra.txt: not covered by SHA256SUMS".to_owned()));
    fs::remove_dir_all(&copy).unwrap();
}

/// P4: the module carries the pinned PolyForm Noncommercial 1.0.0 text and
/// the owner's Required Notice, and so does every export (checked in
/// `export_builds_standalone`).
/// Fail-without mutant: change one character of `NOTICE`.
#[test]
fn license_and_notice_are_pinned() {
    assert_eq!(export::check_license(&module_root()), Vec::<String>::new());
    let manifest = fs::read_to_string(module_root().join("Cargo.toml")).unwrap();
    assert!(manifest.contains("license = \"PolyForm-Noncommercial-1.0.0\""));
}
