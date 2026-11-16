# Done list — slice-14 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-14 --dispatch` (main 2fca3dc8c17dda8733b2bfe8748584da60d2b627, plan rev 6).
Scope: server-side checks are first-class runs — the SLICE-14-DEV repair node (plans/2026-11-06-integration/dispatch-graph.json), ordered by A/EXEC-PM after HOLDOUT-HC3 failed run 2 (public consequence: "The owner mandate is not kept. The test is not bound to the commit under test, and effectiveness is not measured."; HOLDOUT-HC2 was ACCEPTED on run 2). Branch `slice-14`, cut from main 2fca3dc.
Resource scope (from the graph contract, verbatim): `crates/holden-mcp/src/workflow*.rs`, `crates/holden-embed/**` (agent, jit, embed), `crates/holden-ledger/**`, `crates/holden-cli/src/**` (trace/replay lookup only), tests, PLUS `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-14"`, bump the array length). AMENDMENT 1 (derived grants): `crates/holden-mcp/src/server.rs` (named-commit dispatch at the composition layer — P1's binding is enforced where the server composes the check; the graph's `workflow*.rs` wording named the workflow entry, but the ruling's requirement lives one file over) and `crates/holden-mcp/src/ledger.rs` (per-check signed provenance glue — P2's ledger appends for server-created runs require the server's ledger shim; the holden-ledger crate itself is already granted).
Behavior (fixed by the ruling): every just-in-time check the server composes for an implementer (W2, /holden check) is a FIRST-CLASS holden run. (1) It evaluates exactly the commit under test — the client HEAD at request time, or a commit the caller names — NEVER a commit holden itself created; holden never commits embed or approve output. (2) The run is appended to the signed ledger so `holden trace`, `holden replay` and `audit_reveal` work for it. (3) Telemetry (W4 sample sizes and catch rates) and the feedback weights count it.
Fixtures: build your own synthetic fixtures for the client side. Do NOT guess or imitate the holdout clients.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 commit binding: a W2 check through /holden on a client evaluates the client HEAD commit; if embed or approve wrote files, they are never committed by holden and never become the evaluated commit.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: evaluating a holden-made commit — the test must go red.
   - Test: `holden-mcp::tests::w2_check_evaluates_the_commit_under_test`
2. P2 ledger traceability: `holden trace <run>` and `holden replay <run>` succeed for a W2 run created through the MCP server, and `audit_reveal` verifies its parameters.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: skipping the ledger append — the test must go red.
   - Test: `holden-cli::tests::server_w2_run_traces_replays_and_reveals`
3. P3 effectiveness counted: after N W2 runs, W4 status reports those N runs in the catch-rate sample sizes and the feedback weights change.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: telemetry ignoring server-side runs — the test must go red.
   - Test: `holden-mcp::tests::w4_counts_server_side_runs`
4. P4 platform parity: P1-P3 hold on macOS AND Linux with identical test names.
   - Reopen: the host gate (macOS) and the VM gate record compared by name sets; the QA verifies the comm itself.
   - Fail-without mutant: none beyond P1-P3's (this is the parity property over the same tests).
   - Test: the P1-P3 names, identical on both platforms.
Gates (exact commands):
- `cargo xtask gate slice-14`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-14 --linux-vm` (run by the orchestrator, outer alarm 2700, after the merge-main + pump-grep ritual)
Out of scope (recorded obligations, not fixed here): everything not listed above; the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the behavior is fixed by the ruling.
