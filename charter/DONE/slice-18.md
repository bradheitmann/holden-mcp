# Done list — slice-18 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-18 --dispatch` (main f34fedea7b3ace01cee140b938a36238de74b559, plan rev 12).
Scope: declared crate paths — bind Cargo crates that are not at the repository root (the SLICE-18-DEV repair node, plans/2026-11-06-integration/dispatch-graph.json plan rev 10/12). Branch `slice-18`, cut from main f34fede.
Resource scope (from the graph contract, verbatim): `crates/holden-eval/src/product.rs` and `binding.rs`, `crates/holden-embed/**` (embed options and host config), `crates/holden-cli/src/**` (embed flags), `fixtures/clients/**` (NEW synthetic fixtures for the declared-crate hosts), tests, PLUS `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-18"`, bump the array length), PLUS AMENDMENT 1 (derived grant, A/EXEC-PM-correction pattern): `crates/holden-eval/src/**` — the evaluator's build/probe path (at minimum `evaluate.rs`, which hardcodes the evaluated build to `src/Cargo.toml` and performs a single build); the ruling's behavior — each declared manifest binds with its own build and binary entry points and W2 completes on it — cannot exist without the evaluator build path; the graph's two-file wording named the binding surface but the behavior lives in the evaluator.
Behavior (fixed by the ruling): a host whose root has no Cargo.toml can DECLARE one or more Cargo manifests below the root (for example app/Cargo.toml, tools/<name>/Cargo.toml, crates/odin-say/Cargo.toml — via a new embed flag, `--manifest <relative-path>`, repeatable). holden binds each declared one with its own build and binary entry points. A host with NEITHER a root manifest NOR a declared one is still refused, with an exact, documented refusal naming the fix. NOTE: crates/holden-eval/src/binding.rs already carries a `ProductBinding::Declared` variant (slice-11 groundwork) — this slice makes it reachable, correct and proven.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 declared single crate binds and runs: a fixture host with no root Cargo.toml and a crate under app/ binds when declared (--manifest app/Cargo.toml) and W2 completes on it.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: ignore the declaration — the test must go red with UnsupportedStack.
   - Test: `holden-cli::tests::declared_app_crate_binds_and_completes`
2. P2 two declared crates bind separately: two declared crates (tools/a, tools/b) in one host bind separately, each with its own entry points and verdicts.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: bind only the first declared crate (drop the second) — the test must go red.
   - Test: `holden-cli::tests::two_declared_crates_bind_separately`
3. P3 the undeclared refusal is exact and documented: an undeclared non-root host is refused with an exact, documented refusal text that names --manifest; a test pins the text.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: change the refusal text (or drop the --manifest mention) — the pinning test must go red.
   - Test: `holden-cli::tests::undeclared_non_root_refusal_names_manifest`
4. P4 declared paths are validated: relative only — no absolute path, no escape outside the host, no crate that cargo metadata reports outside the declared manifest; a workspace nested elsewhere (like a separate wasm workspace) is left alone.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: accept an absolute or escaping --manifest value — the test must go red.
   - Test: `holden-cli::tests::declared_paths_are_validated`
5. P5 platform parity: macOS and Linux gates are green with identical test names.
   - Reopen: the host gate (macOS) and the VM gate record compared by name sets; the QA verifies the comm itself.
   - Fail-without mutant: none beyond P1-P4's (the parity property over the same tests).
   - Test: the P1-P4 names, identical on both platforms.
6. P6 help exit codes: `holden --help`, `holden help` and `holden <command> --help` print the usage to stdout and exit 0; a real usage error still exits 2; a test pins both exit codes.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: send usage to stderr (or exit non-zero on --help) — the test must go red.
   - Test: `holden-cli::tests::help_exit_codes_are_pinned`
7. P7 the --lens default is cwd-independent: the default lens ships inside the install, and the documented embed command works from any host directory.
   - Reopen: fresh-clone re-run on macOS (run the documented embed command from a DIFFERENT directory than the host root); the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: restore the cwd-relative default — the test must go red with entity not found.
   - Test: `holden-cli::tests::lens_default_is_cwd_independent`
8. P8 a refused embed leaves no host state behind: a test runs refuse, fix, re-run and gets the same host state as a clean first run (no half-made lens directory, no orphan keys).
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: leave the partial state on refusal — the test must go red.
   - Test: `holden-cli::tests::refused_embed_leaves_no_host_state`
9. P10 the usage and README document the embed contract: the post-commit hook is shared by every worktree of the host; how planning is detected when the host has charter/ but no PLANNING.md; whether embed or the post-commit jit runs cargo build or test in the host; the --state default.
   - Reopen: fresh-clone re-run on macOS; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: remove one of the documented statements — the test must go red.
   - Test: `holden-cli::tests::embed_usage_documents_the_contract`
Gates (exact commands):
- `cargo xtask gate slice-18`
- `HOLDEN_VM_ADDR=holden-boat HOLDEN_VM_USER=user cargo xtask gate slice-18 --linux-vm` (run by the orchestrator on Boat gate-19, x86_64, outer alarm 2700, after the merge-main + pump-grep ritual; the Tart VM 192.168.64.10 arm64 is the fallback; check Boat's auto-stop window with A/EXEC-PM if a run would cross it)
Out of scope (recorded obligations, not fixed here): everything not listed above; planted-fault controls (slice-19); the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the behavior is fixed by the ruling.
DISK + DELETION RULES (binding): ONE CARGO_TARGET_DIR for ALL slice-18 proof runs (/tmp/holden-s18-target, reused); mutants get ONE scratch target (/tmp/holden-s18-mutant) deleted the moment its proof ran; NO other new /tmp/holden-* folder; EVERY rm names a literal path or "${var:?}" — NEVER a glob; fixture temp dirs are Drop-guard cleaned (the slice-17 P7 pattern).
