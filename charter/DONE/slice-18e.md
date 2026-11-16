# Done list — slice-18e (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-18e --dispatch` (main 27e47f6, plan rev 25 content).
Scope: command output a seat can act on — the SLICE-18E-DEV node (plans/2026-11-06-integration/dispatch-graph.json plan rev 25; slice-18e runs BESIDE slice-18d and depends on slice-18b only). Branch `slice-18e`, cut from main 27e47f6.
Resource scope (from the graph contract, verbatim): `crates/holden-cli/src/**` (usage, help and output), `crates/holden-embed/src/embed.rs` (the embed report only), `crates/holden-cognition/src/adapters.rs` (the charter planning adapter only), README, tests.
AMENDMENT 0 (derived grant, pre-included at dispatch, the slice-18b amendment-0 pattern): `fixtures/clients/**` — P4 needs a host with a stale cognition file and P6 needs a DONE file carrying a numbered Steps: list before its Proofs: list; a committed fixture is the cleanest pin for both (in-test constructed hosts remain allowed where simpler).
Behavior (fixed by the ruling): wave-1 examiners found a successful embed prints no next step although the exact owner command (`holden approve --host .`) is written into `.holden/promise-proposals.json`; `holden <command> --help` prints the whole 76-line usage; bare `holden` says unknown command; `holden ledger verify` prints the ledger as an absolute path. Client checks of slice 18b found an embed deletes stale cognition files from the host tree and prints nothing for them (1,184 files on one host), and a source document with no candidates (charter/INTENT.md, by design: it states goals, not numbered proofs) looks the same as a parse failure. A third client found numbered lines outside the proof list taken as promises: one DONE file lists its seven gate steps as a numbered list under Steps: before its Proofs: list. After this slice each command says what to do next and prints no machine path.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 a successful embed ends its stdout with the next steps: the exact owner command from promise-proposals.json, marked as the owner only, and the holden agent serve --host command.
   - Reopen: fresh-clone re-run on macOS, BOUNDED, only as the orchestrator's single final gate (see Gates); the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: drop the next-steps lines — the test goes red.
   - Test: `successful_embed_prints_next_steps_from_promise_proposals`.
2. P2 holden <command> --help prints only that command usage and exits 0, for every command; bare holden prints the global usage and exits 2 with no unknown command line.
   - Reopen: as P1.
   - Fail-without mutant: make --help print the whole usage — the per-command pin goes red.
   - Tests: `per_command_help_prints_only_that_command`, `bare_holden_prints_global_usage_exits_2`.
3. P3 holden ledger verify and holden ledger telemetry print the ledger as a path relative to the private root.
   - Reopen: as P1.
   - Fail-without mutant: print the absolute path — the sentinel-home scan goes red.
   - Test: `ledger_paths_are_relative_to_the_private_root`.
4. P4 every file an embed removes from the host tree prints one `removed: <host-relative path>` line, and the report ends with the count of files written, unchanged and removed; nothing is removed without its line.
   - Reopen: as P1.
   - Fail-without mutant: remove a file without printing its line — the test goes red.
   - Test: `removed_files_print_lines_and_the_report_counts_them`.
5. P5 a source document that yields no candidates is named in the report with the reason (for charter/INTENT.md: the intent card states goals, not numbered proofs; candidates come from DONE proofs and GUARANTEES).
   - Reopen: as P1.
   - Fail-without mutant: drop the no-candidates line — the test goes red.
   - Test: `source_without_candidates_is_named_with_reason`.
6. P6 in a charter DONE list only the numbered lines of its proof list (the block that follows a line starting `Proofs`) are candidates; numbered lines under any other label or heading (Steps:, Gates, Out of scope) are not; the slice-18b P1 fixture yields the same candidates as before.
   - Reopen: as P1.
   - Fail-without mutant: accept numbered lines outside the Proofs block — the Steps-list fixture test goes red.
   - Test: `only_the_proofs_block_yields_charter_candidates`.
7. P7 macOS and Linux gates are green with identical test names (the P1-P6 names, identical on both platforms).
   - Reopen: the host gate record (the orchestrator's single final bounded macOS gate) and the VM gate record compared by name sets; the QA verifies the comm itself.
   - Fail-without mutant: none beyond P1-P6's (the parity property over the same tests).
   - Test: the P1-P6 names, identical on both platforms.
Gates (exact commands — the 2026-11-15 speed order, binding):
- ALL mutant runs and repeat runs run on Boat, not the Mac: the worker keeps ONE Boat workspace clone of the lane (staged by git bundle, the cross-platform method) and runs every mutant round there with `CARGO_TARGET_DIR=/tmp/holden-s18e-mutant` ON BOAT, deleted after each proof (literal path). A mutant whose test is not platform-specific is proven ONCE, on Boat.
- On the Mac the worker runs ONLY focused tests (`cargo test -p <crate> --test <file>`, bounded CARGO_BUILD_JOBS=4 / RUST_TEST_THREADS=2) into /tmp/holden-s18e-target.
- The worker runs NO final macOS gate and NO VM gate. The orchestrator runs, per lane, after the merge-main + re-dispatch: first `HOLDEN_VM_ADDR=holden-boat HOLDEN_VM_USER=user cargo xtask gate slice-18e --linux-vm` on Boat gate-19 (x86_64; fresh clone; outer alarm 2700; the Tart VM 192.168.64.10 arm64 is the fallback), then the SINGLE final bounded macOS gate `CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=2 cargo xtask gate slice-18e` — one Mac gate at a time ACROSS both lanes (18d and 18e; the orchestrator serializes them).
- Boat runs use the worker's OWN clone and target dir (never the gate driver's remote directories), so worker runs and orchestrator VM gates do not collide; the orchestrator serializes its own VM gates.
Budget (from the graph): qa_attempts 3, repair_cycles 1.
Out of scope (recorded obligations, not fixed here): everything not listed above; slice-18c; slice-20 (waits for 18c AND 18e); the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
PARALLEL-LANE RULES (binding): 18d changes NONE of this slice's source files (its scope: state.rs, distribution.rs, holden-mcp/src/**, xtask gate checks); any test-file overlap between the lanes is a merge at the second lane's gates, not a design conflict — never edit the other lane's worktree, and keep every change inside THIS slice's resource scope.
DISK + DELETION RULES (binding): the Mac target is /tmp/holden-s18e-target (focused tests only); the mutant scratch is /tmp/holden-s18e-mutant ON BOAT (deleted the moment its proof ran); NO other new /tmp/holden-* folder on either machine; EVERY rm names a literal path or "${var:?}" — NEVER a glob; never touch any `holden-<uid>-*` socket directory (501 on the Mac, 1000 on Boat); after every run, remove THAT run's holden-*/hm11-* scratch by literal path before the next step.

AMENDMENT 1 (2026-11-16, orchestrator, post-DEV derived grant): `xtask/src/gate.rs` — the `SLICES` array only: add `"slice-18e"` and bump the array length. Derived: a slice whose gate is not registered cannot be gated at all — `cargo xtask gate slice-18e` exits 2 ("no gate for that slice") without the registration; pure mechanical necessity, the same grant every prior slice carried (slice-18b's DONE: "PLUS xtask/src/gate.rs (the SLICES array only)").
