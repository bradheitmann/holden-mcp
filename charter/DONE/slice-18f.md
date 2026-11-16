# Done list — slice-18f (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-18f --dispatch` (main ab1f1d4, plan rev 26 content).
Scope: proof lists under markdown headings — the SLICE-18F-DEV node (plans/2026-11-06-integration/dispatch-graph.json plan rev 26; slice-18f runs BESIDE slice-18d and depends on slice-18e, which is accepted). Branch `slice-18f`, cut from main ab1f1d4. This slice repairs an 18e regression: a client host labels its DONE proof lists with markdown headings instead of a bare `Proofs` line, and its cognitions fell from 1,524 to 235 (246 of 292 sources reported no candidates). The P6 wording was the plan author's error, not the DEV's.
Resource scope (from the graph contract, verbatim): `crates/holden-cognition/src/adapters.rs` (the charter planning adapter only), `fixtures/clients/**`, `tests`.
STANDING GRANT (pre-included at dispatch, the lesson of slice-18e's gate refusal): `xtask/src/gate.rs` — the `SLICES` array only: add `"slice-18f"` and bump the array length. A slice whose gate is not registered cannot be gated; pure mechanical necessity, the same grant every prior slice carried.
Behavior (fixed by the ruling): slice-18e P6 took candidates only from the block after a line starting with the word Proofs. A client host labels its DONE proof lists with markdown headings instead (`## Proofs` in 78 files, `## Fixed proof groups` in 77, `## Proofs (REV-4 ...)` in 14, `## Proof 1 — <title>` in 6). After this slice every common form of proof label is read, and gate steps are still excluded.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 a proof block starts at a line starting `Proofs` (as now) or at a markdown heading whose text contains the word proof or proofs, case-insensitive (`## Proofs`, `## Proofs (REV-4 ...)`, `## Fixed proof groups`, `## Proof 1 — <title>`); it ends at the next heading WITHOUT that word or at a top-level non-list line; a fixture DONE file per form pins each, and each yields exactly its numbered proofs.
   - Reopen: fresh-clone re-run on macOS, BOUNDED, only as the orchestrator's single final gate (see Gates); the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: revert the block-start rule to the bare `Proofs` line only — every heading-form fixture test goes red.
   - Test: `proof_blocks_start_at_proofs_lines_and_markdown_headings`.
2. P2 inside a proof block, top-level bulleted items (`-` or `*`) are candidates as well as numbered ones; indented sub-items (`Reopen:`, `Fail-without mutant:`, `Test:`) are never candidates.
   - Reopen: as P1.
   - Fail-without mutant: drop top-level bullets from candidacy (or admit indented sub-items) — the pin goes red.
   - Test: `top_level_bullets_are_candidates_indented_subitems_are_not`.
3. P3 the 18e decoys stay excluded: numbered lists under `Steps:`, `Gates`, `Out of scope` or any heading without the word proof are not candidates; the 18b and 18e fixtures yield the same candidates as before.
   - Reopen: as P1.
   - Fail-without mutant: accept decoy sections — the pin goes red.
   - Test: `decoy_sections_stay_excluded_and_older_fixtures_hold`.
4. P4 macOS and Linux gates are green with identical test names (the P1-P3 names, identical on both platforms).
   - Reopen: the host gate record (the orchestrator's single final bounded macOS gate) and the VM gate record compared by name sets; the QA verifies the comm itself.
   - Fail-without mutant: none beyond P1-P3's (the parity property over the same tests).
   - Test: the P1-P3 names, identical on both platforms.
Gates (exact commands — the 2026-11-15 speed order, binding):
- ALL mutant runs and repeat runs run on Boat, not the Mac: the worker keeps ONE Boat workspace clone of the lane (staged by git bundle, the cross-platform method) and runs every mutant round there with `CARGO_TARGET_DIR=/tmp/holden-s18f-mutant` ON BOAT, deleted after each proof (literal path). A mutant whose test is not platform-specific is proven ONCE, on Boat.
- On the Mac the worker runs ONLY focused tests (`cargo test -p <crate> --test <file>`, bounded CARGO_BUILD_JOBS=4 / RUST_TEST_THREADS=2) into /tmp/holden-s18f-target.
- The worker runs NO final macOS gate and NO VM gate. The orchestrator runs, after the merge-main + re-dispatch: first `HOLDEN_VM_ADDR=holden-boat HOLDEN_VM_USER=user cargo xtask gate slice-18f --linux-vm` on Boat gate-19 (x86_64; fresh clone; outer alarm 2700; the Tart VM 192.168.64.10 arm64 is the fallback), then the SINGLE final bounded macOS gate `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=2 cargo xtask gate slice-18f` — one Mac gate at a time ACROSS all lanes (the orchestrator serializes them).
- Boat runs use the worker's OWN clone and target dir (never the gate driver's remote directories).
Budget (from the graph): qa_attempts 3, repair_cycles 1.
Out of scope (recorded obligations, not fixed here): everything not listed above; slice-18c; slice-20 (waits for 18c AND 18e — 18e is accepted); the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
PARALLEL-LANE RULES (binding): the 18d DEV worker is running concurrently in /tmp/holden-mcp-dev-slice-18d on OTHER files (state.rs, distribution.rs, holden-mcp/src/**, xtask gate checks) — never touch that worktree; 18f changes NONE of 18d's source files (adapters.rs and fixtures only); keep every change inside THIS slice's resource scope; do NOT merge main yourself.
DISK + DELETION RULES (binding): the Mac target is /tmp/holden-s18f-target (focused tests only); the mutant scratch is /tmp/holden-s18f-mutant ON BOAT (deleted the moment its proof ran); NO other new /tmp/holden-* folder on either machine; EVERY rm names a literal path or "${var:?}" — NEVER a glob; never touch any `holden-<uid>-*` socket directory (501 on the Mac, 1000 on Boat); after every run, remove THAT run's holden-*/hm11-* scratch by literal path before the next step.
