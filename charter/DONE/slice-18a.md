# Done list — slice-18a (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-18a --dispatch` (main f32b142d9264e814f0d33fe06ae47eadf68f8c31, plan rev 11).
Scope: a refused embed still prints the detection report — the SLICE-18A-DEV repair node (plans/2026-11-06-integration/dispatch-graph.json, plan rev 10/11). Branch `slice-18a`, cut from main f32b142.
Resource scope (from the graph contract, verbatim): `crates/holden-embed/src/embed.rs` (report assembly and the refusal path), `crates/holden-cli/src/embed_cmd.rs` (printing), tests, PLUS `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-18a"`, bump the array length — the dispatch-infrastructure grant every slice carries).
Behavior (fixed by the ruling): planning detection already runs before the binding check in `holden embed`. When a later step refuses (unsupported stack, lens, forge), the caller STILL gets the detection report — the planning process found, its source documents, and the count and text of candidate promises — printed BEFORE the refusal line, in the same shape as the report on success. Client wave gates read it. The exit code stays 1.
Fixture: the existing `fixtures/clients/prose-markdown` fixture (planning promises; NO root Cargo.toml) is the P1 host — no new fixture is needed.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 the report survives a refusal: the prose-markdown fixture host (planning promises, no root Cargo.toml) is refused with unsupported_stack, and its output carries the detection report (planning process, source documents, candidate promise count and texts) BEFORE the refusal line; the exit code stays 1.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: drop the report on the refusal path — the test must go red.
   - Test: `holden-cli::tests::refused_embed_still_prints_detection_report`
2. P2 shape parity: the report on a refusal has the same fields, in the same order, as the report on success; a test compares the two shapes.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: reorder or drop a field on the refusal path's report — the shape comparison must go red.
   - Test: `holden-cli::tests::refusal_report_matches_success_shape`
3. P3 no-promise refusals unchanged: a host with no planning promises is refused as before, with no report, and its refusal text is unchanged.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: print a report (even an empty one) on the no-promise refusal path — the test must go red.
   - Test: `holden-cli::tests::no_promise_refusal_prints_no_report`
4. P4 no machine values: the report carries no machine path and no sealed value.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: include the socket or state path in the report — the test must go red.
   - Test: `holden-cli::tests::detection_report_carries_no_machine_values`
5. P5 platform parity: macOS and Linux gates are green with identical test names.
   - Reopen: the host gate (macOS) and the VM gate record compared by name sets; the QA verifies the comm itself.
   - Fail-without mutant: none beyond P1-P4's (the parity property over the same tests).
   - Test: the P1-P4 names, identical on both platforms.
Gates (exact commands):
- `cargo xtask gate slice-18a`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-18a --linux-vm` (run by the orchestrator, outer alarm 2700, after the merge-main + pump-grep ritual)
Out of scope (recorded obligations, not fixed here): everything not listed above; declared crate paths (slice-18); the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the behavior is fixed by the ruling.
DISK + DELETION RULES (binding): ONE CARGO_TARGET_DIR for ALL slice-18a proof runs (/tmp/holden-s18a-target, reused); mutants get ONE scratch target (/tmp/holden-s18a-mutant) deleted the moment its proof ran; NO other new /tmp/holden-* folder; EVERY rm names a literal path or "${var:?}" — NEVER a glob; fixture temp dirs are Drop-guard cleaned (the slice-17 P7 pattern).
