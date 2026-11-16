# Done list — slice-15 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-15 --dispatch` (main 6af6628baf676b3ccfe52895c0e75967e3cf15b4, plan rev 7).
Scope: dependency advisory RUSTSEC-2026-0462 (rmcp < 3.9.0, medium 6.5) — the SLICE-15-DEV repair node (plans/2026-11-06-integration/dispatch-graph.json). Branch `slice-15`, cut from main 6af6628.
Resource scope (from the graph contract, verbatim): `Cargo.toml` / crate `Cargo.toml` rmcp pins, `Cargo.lock`, any rmcp 3.9 API adaptations in `crates/holden-mcp`, `crates/holden-embed` and `crates/holden-cli`, plus tests. AMENDMENT 1 (derived grant): `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-15"`, bump the array length) — without the registration `cargo xtask gate slice-15` exits 2 ("no gate for that slice") and NO gate proof can execute; the same dispatch-infrastructure grant every slice since 12b carries. (No other files; no behaviour change.)
Behavior (fixed by the ruling): bump the rmcp pins to the current 3.9.x line — EXACT pin as now (`=3.9.<current patch>`, the workspace pin at `Cargo.toml` with all crates on `workspace = true`), update `Cargo.lock`, adapt any API changes the minor bump introduces, and make NO behaviour change. The advisory must clear for cargo-deny, cargo-audit and the guard on main.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 guard clean: `cargo xtask guard` verdict Pass on the candidate (cargo_deny and cargo_audit clean of RUSTSEC-2026-0462).
   - Reopen: run the guard in a fresh clone on macOS; the Linux half via the VM gate record; the QA re-runs the guard independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: re-pin `=3.8.0` — the guard must REFUSE naming RUSTSEC-2026-0462.
   - Test: the guard run itself (the executable form of the proof).
2. P2 gates green with identical names: the full host gate and the Linux VM gate are green with the same test names as main before the bump (no test removed, none renamed).
   - Reopen: both gates at the candidate tip (records name the commit); the QA verifies the name sets against the pre-bump records (host 20261108T150008Z-macos.json 182 names, VM host-copy 20261108T151334Z-linux-vm.json 184 names) by comm and compares.
   - Fail-without mutant: none beyond P1's (the name-identity property is checked by comparison; a removed test would break it).
   - Test: `cargo xtask gate slice-15` (host) and its `--linux-vm` run.
3. P3 product regression: the real-repo e2e (4/4) and the workflows tests still pass.
   - Reopen: fresh-clone re-run on macOS (the real-repo tests serially against the sacred seed); the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: none beyond P1's (this is the no-behaviour-change property; any behaviour drift breaks these existing tests).
   - Test: the four `real_repo` tests + the `workflows` test target, unchanged names.
Gates (exact commands):
- `cargo xtask gate slice-15`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-15 --linux-vm` (run by the orchestrator, outer alarm 2700, after the merge-main + pump-grep ritual)
Out of scope (recorded obligations, not fixed here): everything not listed above; any behaviour change; any feature addition; GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the behavior is fixed by the ruling.
