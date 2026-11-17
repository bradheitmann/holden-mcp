# Done list — slice-18h (written BEFORE dispatch; every proof carries its own Reopen line and mutant from the start)
Base bytes: the commit recorded by `charter gate slice-18h --dispatch` (main 9d732a2, plan rev 32 content).
Scope: proofs whose head line is a short name, and lists the adapter skipped, are found or reported — the SLICE-18H-DEV node (plan rev 32; slice-18h depends on slice-18g-QA, which is accepted, and runs BESIDE slice-18d). Branch `slice-18h`, cut from main 9d732a2.
Resource scope (from the graph contract, verbatim): `crates/holden-cognition/src/adapters.rs` (the charter planning adapter only), `crates/holden-cli/src/**` (the per-source report line only), `fixtures/clients/**`, `tests`.
STANDING GRANT (pre-included): `xtask/src/gate.rs` — the `SLICES` array only: add `"slice-18h"` and bump the array length (26 on main; 27 after).
Behavior (fixed by the ruling): a re-embed of the first client after 18g gives 1610 cognitions from 287 of 290 DONE sources, and four DONE files with real numbered proofs still give none — three causes: (a) two files write each proof as a short name on the numbered line with the claim in indented sub-items, so the 40-character minimum drops them; (b) one file has its numbered proofs under the title and, far below, a sub-heading whose text contains the word proof (evidence, not a proof) — the word makes the file count as labeled, the labeled block holds no items, and the title list is never read; (c) one file has its numbered proofs under a second-level heading that does not contain the word proof. In all three the embed says only that the source has no candidates. A second client re-embed hid a fourth cause behind cancelling errors: 5 name-only proofs missed (cause a) and 4 design bullets taken because the file's TITLE contains the word proof (a slice named ...-proof-integrity), opening a proof block over a plain Design line.
Proofs (the graph's acceptance criteria, each with its fail-without mutant and its own Reopen line):
1. P1 a proof item whose own text (before its first indented sub-item) is shorter than the 40-character minimum and which has at least one indented sub-item is a candidate; its text and span run from the item through the end of its first sub-item that does not start with Reopen, reopened, Fail-without, Mutant or Test (compared without case), or through its first sub-item when all of them do; a fixture with `1. D1:` and three sub-items, and one with a bold short name, pins the text and the span.
   - Fail-without mutant: keep the 40-character drop for sub-itemed short items — the fixture goes red (the candidates vanish).
   - Reopen: the accept chain's reopen of all proofs (run id and evidence sha256 recorded at acceptance in charter/.freeze/reopen/slice-18h.json); the fresh-clone re-run on macOS is the orchestrator's bounded gate record, the Linux half the VM gate record; the QA re-runs the proof independently and its run is reopened and compared.
   - Test: `short_named_items_extend_through_their_claim_subitems`.
2. P2 an item whose own text reaches the minimum is unchanged: same text, same span, same id as after 18g; the 18b, 18e, 18f and 18g fixture pins stay green WITHOUT EDITS.
   - Fail-without mutant: alter long-item extraction — the unchanged pins go red.
   - Reopen: as P1 (the accept chain's reopen of all proofs; the gate records; the QA's independent run).
   - Test: `long_items_are_unchanged_from_18g`.
3. P3 a short item with no sub-item is still not a candidate, and the report counts it under too short (P6).
   - Fail-without mutant: admit sub-item-less short items — the pin goes red.
   - Reopen: as P1.
   - Test: `short_item_without_subitems_is_counted_too_short`.
4. P4 when a file's labeled proof blocks (label lines and headings containing the word proof) yield no item at all, the adapter reads the file as unlabeled under the 18g title rule: top-level numbered items under the title heading before the first second-level heading; a fixture with title items and a later sub-heading containing the word proof and no items pins them. A file whose labeled block yields at least one item is unchanged.
   - Fail-without mutant: keep the empty-labeled-block verdict — the title items vanish and the test goes red.
   - Reopen: as P1.
   - Test: `labelless_blocks_fall_back_to_the_title_rule`.
5. P5 when a file still yields no item, a top-level numbered list outside code fences in which at least one item has an indented sub-line starting with reopened:, Reopen or Fail-without (compared without case) is a proof list wherever it sits; lists without such a line (Steps, Gates, Schedule, Landing) are still not; a fixture with both under second-level headings pins the proof list in and the decoys out.
   - Fail-without mutant: drop the reopened-subline rule — the proof list vanishes; or admit lists without it — the decoys return.
   - Reopen: as P1.
   - Test: `reopened_sublines_mark_proof_lists_anywhere`.
6. P6 for every DONE source the report states the number of top-level numbered items seen outside code fences, the number taken as candidates, and the number left out by reason (too short, outside any proof list); a source with numbered items and no candidate is listed under its own heading `numbered items found, none taken` with its path and count, DISTINCT from sources with no numbered items; a test pins the three wordings.
   - Fail-without mutant: drop the per-source accounting — the three wordings go unpinned and the test goes red.
   - Reopen: as P1.
   - Test: `per_source_report_counts_items_taken_and_left_out`.
7. P6b the title heading (the first level-1 heading of a DONE file) NEVER opens a proof block because of its words; a slice name containing the word proof does not make the lines under the title a proof block. A fixture whose title is `# Done list — say-proof-integrity`, followed by a plain Design line with four top-level bullets, then a Proofs label with five numbered items, yields exactly the five; the 18g title rule for files with no label is unchanged.
   - Fail-without mutant: let the title's words open a proof block — the four design bullets become candidates and the test goes red.
   - Reopen: as P1.
   - Test: `title_words_never_open_a_proof_block`.
8. P6c a cross-check that compares only totals is not accepted as evidence: every count comparison in the QA of this slice is per file, and the QA names each file where the adapter and the independent counter differ in either direction EVEN WHEN THE TOTALS AGREE.
   - Reopen: the cross-check artifact (per file, both directions) is part of the QA evidence; the QA's independent counter is a plain rule WRITTEN BEFORE IT READS THE ADAPTER (top-level `N.`/`Nb.` lines inside the proof list of each file, judged by reading), with the P7 files checked by hand.
   - Test: the cross-check artifact + the per-file differing list (the QA's evidence; the P1-P7 mutants carry the behavior).
9. P7 the cross-check of 18g P8 is repeated by the QA on the three client clones at named commits, and for the first client at the commit of this finding the four named files yield 5, 6, 6 and 4 candidates; the QA lists every DONE file where numbered items seen exceeds candidates taken, with the reason counts.
   - Fail-without mutant: omit the cross-check artifact or compare totals only — the per-file evidence requirement fails under the P6c rule (the cancelling-errors case is exactly what totals-only hides).
   - Reopen: the cross-check artifact names the commits and the four files' counts (5, 6, 6, 4); the client clones stay READ-ONLY.
   - Test: the cross-check artifact (the QA's evidence).
10. P8 macOS and Linux gates are green with identical test names (the P1-P7 names, identical on both platforms).
   - Fail-without mutant: the P1 mutant (restore the 40-character drop for sub-itemed short items) re-run against the parity test suite — the suite must stay green on both platforms, proving the parity property over the same tests; if any platform's name set diverges, the parity proof fails.
   - Reopen: the host gate record and the VM gate record compared by name sets; the QA verifies the comm itself.
   - Test: the P1-P7 names, identical on both platforms.
Gates (exact commands — the 2026-11-15 speed order + the D179 hold state, binding):
- ALL mutant runs and repeat runs on Boat (own clone, CARGO_TARGET_DIR=/tmp/holden-s18h-mutant ON BOAT, deleted after each proof, literal paths); a non-platform-specific mutant is proven ONCE.
- Mac: the worker runs NO Mac step (the D179 hold; the orchestrator owns the one final bounded macOS gate under the hold rules then in force, with a-holden/MAC-GATE-LOG.md START/END lines).
- The orchestrator: the Boat VM gate first, then the single final bounded macOS gate.
Budget (from the graph): qa_attempts 3, repair_cycles 1.
Out of scope (recorded obligations): everything not listed; slice-22 (after 18c-QA, beside 20); the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
PARALLEL-LANE RULES (binding): the 18d lane edits OTHER files (state.rs, distribution.rs, holden-mcp/src/**, xtask gate checks, quickstart tests) — never touch its worktree or clone; 18h changes NONE of 18d's source files; keep every change inside THIS slice's resource scope; do NOT merge main yourself.
DISK + DELETION RULES (binding): the mutant scratch /tmp/holden-s18h-mutant ON BOAT (deleted the moment its proof ran); NO other new /tmp/holden-* folder on either machine; EVERY rm names a literal path or "${var:?}" — NEVER a glob; never touch any holden-<uid>-* socket directory; per-run scratch cleaned by literal path before the next step.
