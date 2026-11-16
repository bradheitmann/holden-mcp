# Done list — slice-18b (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-18b --dispatch` (main 8316d5b0c45a2693cb0e6996bc00e07f0e4d9143, plan rev 14).
Scope: a charter host yields promises an owner can read and approve — the SLICE-18B-DEV repair node (plans/2026-11-06-integration/dispatch-graph.json plan rev 14; from wave 0's F14: on a charter host the old extraction found 9,869 paragraph-fragment candidates in 291 documents — median 134 characters, 812 of 40 or less — and printed all of them to stderr, 2.2 MB, which an owner approving one by one cannot use). Branch `slice-18b`, cut from main 8316d5b.
Resource scope (from the graph contract, verbatim): `crates/holden-cognition/src/promise.rs`, `crates/holden-cognition/src/adapters.rs` (the charter planning adapter), `crates/holden-embed/src/embed.rs` (report), `crates/holden-cli/src/embed_cmd.rs` (printing), README, tests, PLUS `fixtures/clients/**` (AMENDMENT 0, derived grant: P1 requires a fixture charter host with INTENT + GUARANTEES + three numbered DONE lists + headings + prose — the existing prose-charter fixture lacks GUARANTEES and the extra DONE lists, so the fixture must be extended or added; same derivation pattern as every prior scope amendment), PLUS `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-18b"`, bump the array length).
Behavior (fixed by the ruling): on a charter host (charter/INTENT.md plus charter/DONE/*.md, no PLANNING.md), the candidates are the NUMBERED PROOFS of each DONE list and the GUARANTEES entries, each with its source file and line. Headings, prose paragraphs and lines of 40 characters or less are NOT candidates. Template files (charter/DONE/_template.md and any DONE file whose name starts with _) are not sources. The report prints the count, the count per source document and the first 20 candidates, then the path of the full list inside the agent state (relative to the private root, no machine path); the full list goes to that file. The stderr stays under 64 KB even on a host with 10,000 candidates. Hosts with PLANNING.md and the existing planning fixtures yield the same candidates as before. On a refused embed (the slice-18a path) the same shortened report prints before the refusal line.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 charter candidates are the numbered proofs + guarantees: a fixture charter host (INTENT, GUARANTEES, three DONE lists with numbered proofs, headings and prose) yields exactly its numbered proofs and guarantees as candidates, each with source path and line; headings, prose paragraphs and lines of 40 characters or less are not candidates.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: restore the paragraph extraction — the test must go red on the count.
   - Pin: template files (charter/DONE/_template.md and any DONE file whose name starts with _) are not sources — a test pins it.
   - Test: `holden-cli::tests::charter_yields_numbered_proofs_and_guarantees`
2. P2 the report is short and the full list is in the agent state: the report prints the total, the count per source document and the first 20 candidates, then the path of the full list inside the agent state (relative to the private root, no machine path); the stderr stays under 64 KB on a host with 10,000 candidates.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: print every candidate — the size test must go red.
   - Test: `holden-cli::tests::report_prints_first_20_and_writes_the_full_list`
3. P3 planning-fixture compatibility: hosts with PLANNING.md and the existing planning fixtures yield the same candidates as before.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: alter the PLANNING.md extraction — the pinning test must go red.
   - Test: `holden-cli::tests::planning_hosts_yield_the_same_candidates`
4. P4 the slice-18a path carries the same shortened report: on a refused embed the same shortened report prints before the refusal line.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: drop the report on the refusal path — the test must go red.
   - Test: `holden-cli::tests::refused_embed_prints_the_shortened_report`
5. P5 platform parity: macOS and Linux gates are green with identical test names.
   - Reopen: the host gate (macOS) and the VM gate record compared by name sets; the QA verifies the comm itself.
   - Fail-without mutant: none beyond P1-P4's (the parity property over the same tests).
   - Test: the P1-P4 names, identical on both platforms.
Gates (exact commands):
- `cargo xtask gate slice-18b`
- `HOLDEN_VM_ADDR=holden-boat HOLDEN_VM_USER=user cargo xtask gate slice-18b --linux-vm` (run by the orchestrator on Boat gate-19, x86_64, outer alarm 2700, after the merge-main + pump-grep ritual; the Tart VM 192.168.64.10 arm64 is the fallback; check Boat's auto-stop window with A/EXEC-PM if a run would cross it)
Out of scope (recorded obligations, not fixed here): everything not listed above; slice-18c (one agent state belongs to one host); the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the behavior is fixed by the ruling.
DISK + DELETION RULES (binding): ONE CARGO_TARGET_DIR for ALL slice-18b proof runs (/tmp/holden-s18b-target, reused); mutants get ONE scratch target (/tmp/holden-s18b-mutant) deleted the moment its proof ran; NO other new /tmp/holden-* folder; EVERY rm names a literal path or "${var:?}" — NEVER a glob; fixture temp dirs are Drop-guard cleaned (the slice-17 P7 pattern).

AMENDMENT 1 (2026-11-15, orchestrator, post-DEV derived grant): `crates/holden-cognition/src/lib.rs` — inline `mod tests` ONLY. The slice-13-era prose-charter expectations in that module must follow this slice's granted behavior (on charter documents the numbered DONE proofs and the GUARANTEES entries are the candidates, not prose paragraphs; charter hosts carry DiscoveryKind::Explicit; prose-markdown keeps DiscoveryKind::Prose) and the amendment-0 fixture change (fixtures/clients/prose-charter/charter/DONE/slice-1.md gains its numbered "P1 source proof:" line). Without these test updates the granted behavior cannot be pinned by the crate's own tests. No production code in lib.rs changes. Same derivation pattern as slice-14's server.rs and slice-17's workflow.rs grants.
