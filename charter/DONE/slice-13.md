# Done list — slice-13 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-13 --dispatch` (main 0170606ae96905bec22b51ad308f29cbb5873cff, plan rev 5).
Scope: promise discovery from prose — the SLICE-13-DEV repair node (plans/2026-11-06-integration/dispatch-graph.json), ordered by A/EXEC-PM after HOLDOUT-HC2 and HOLDOUT-HC3 failed run 1 (public consequences: "Holden does not follow another project own plan, and the single command does not carry the work."; "The owner mandate is not kept. A test does not come from the owner words and the situation, and the tool is not carried into the other project."). Branch `slice-13`, cut from main 0170606.
Resource scope (from the graph contract, verbatim): `crates/holden-cognition/**` (planning adapters), `crates/holden-embed/**` (embed flow), `crates/holden-mcp/src/workflow*.rs` (W1 onboard proposal), tests, `fixtures/clients/**`, PLUS `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-13"`, bump the array length).
Behavior (fixed by the ruling): where promise discovery today works only with explicit markers and REFUSES prose projects (charter INTENT paragraphs, README, PRD, docs), it must instead PROPOSE candidate promises quoted byte-exact (with byte offsets) from the project's own words, for owner approval. Embed then COMPLETES in AWAITING_OWNER_APPROVAL: the detection report names the planning process found, the harness connection and /holden are written, and the proposal file is written. NO check runs before approval. After the owner approves (real PTY in tests), W2 runs.
Fixtures: build synthetic fixtures for BOTH cases (explicit-marker clients AND prose-only clients) under `fixtures/clients/**`. Do NOT guess or imitate the holdout clients.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 charter-prose: a synthetic charter client with only prose INTENT paragraphs gets verbatim candidate promises proposed (byte offsets), with no refusal.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: the marker-only extraction — the test must go red.
   - Test: `holden-cognition::tests::prose_charter_yields_verbatim_candidates`
2. P2 markdown-prose: a synthetic plain-markdown client with prose README/docs gets the same; the detection report names markdown.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: the marker-only extraction — the test must go red.
   - Test: `holden-cognition::tests::prose_markdown_yields_candidates_and_names_markdown`
3. P3 awaiting-owner completion: embed completes in AWAITING_OWNER_APPROVAL — the harness connection, /holden and the proposal file are written, and no check runs before approval.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: run a check on unapproved candidates — the test must go red.
   - Test: `holden-embed::tests::embed_completes_awaiting_owner_approval`
4. P4 approval enables W2: after `holden approve` (owner, real PTY in tests), W2 runs a JIT check whose plan traces to an approved verbatim promise; the real-repo regression stays 4/4 green.
   - Reopen: fresh-clone re-run on macOS (the approval through a real PTY); the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: approve a NON-verbatim (paraphrased) candidate — the trace/W2 assertion must fail.
   - Test: `holden-embed::tests::owner_approval_enables_w2_on_approved_promise`
5. P5 never paraphrase: every proposed promise is a byte-exact substring of the source file, with correct offsets.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: trimmed or normalized text in the proposal — the test must go red.
   - Test: `holden-cognition::tests::proposals_are_byte_exact_substrings`
Gates (exact commands):
- `cargo xtask gate slice-13`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-13 --linux-vm` (run by the orchestrator, outer alarm 2700, after the merge-main + pump-grep ritual)
Out of scope (recorded obligations, not fixed here): everything not listed above; the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the behavior is fixed by the ruling.
