# Done list — slice-9 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-9 --dispatch`
Scope: client planning-process adapters — holden reads a client repo's planning process and feeds it: promises come from the planning artifacts the repo already uses, and holdout checkpoints are written back into that process in its own format. Branch `slice-9`, cut from slice-8 at 18c7265 under the overnight SPECULATIVE_DISPATCH ruling (HC-1 still gates acceptance; VM gates deferred while VM_OFF). Resource scope: `crates/holden-cognition` (planning adapters) or a new `crates/holden-plan-adapters` (if the new crate is chosen, a one-line workspace-membership edit in the root Cargo.toml is authorized for exactly that), `fixtures/clients/*` (synthetic client repos), tests. Additionally authorized as dispatch infrastructure: registering "slice-9" in `xtask/src/gate.rs::SLICES` (that file, that array only — no other xtask change). MUST NOT touch `crates/holden-cli` (slice-10's lane).
Proofs:
1. P1 adapter-detection: detection among charter/ (INTENT + DONE lists), a dispatch-graph.json client (plangraph-shaped or an equivalent planning-graph format), and plain markdown planning (README/PRD/docs) is deterministic; an unknown process falls back to markdown with a typed limitation.
   - Reopen: fresh-clone re-run at the slice tip on macOS, and in the Linux VM gate with identical test names when the VM returns.
   - Fail-without mutant: make detection order-dependent (first-match by map iteration). The detection test must fail.
   - Test: `holden-plan-adapters::tests::{detection_is_deterministic, unknown_process_falls_back_with_typed_limitation}` (crate name follows the adapter-crate choice; keep these test paths exact)
2. P2 promises-verbatim: promise extraction from each detected process keeps the byte-offset verbatim rule (no paraphrase).
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: normalize whitespace in an extracted promise. The verbatim test must fail.
   - Test: `holden-plan-adapters::tests::promise_extraction_is_verbatim_byte_offset`
3. P3 writeback-valid: for a dispatch-graph client, emitted HOLDOUT-* nodes (type HOLDOUT, depends_on the client's QA nodes) still pass the client's own validate-graph check; for a charter client, a holdout proof line lands in the next DONE list template; an already-pinned DONE list is never edited.
   - Reopen: fresh-clone re-run; the emitted graph is validated with the client's own jq validator in the test.
   - Fail-without mutant: emit HOLDOUT nodes with depends_on pointing at DEV nodes. The writeback validation test must fail.
   - Test: `holden-plan-adapters::tests::{graph_writeback_passes_client_validation, charter_writeback_never_touches_pinned_lists}`
4. P4 writeback-proposal-only: write-back is proposal-only by default (a patch file), applied only by an explicit flag; tests prove pinned files are untouched.
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: apply the write-back in place by default. The proposal-only test must fail.
   - Test: `holden-plan-adapters::tests::writeback_is_proposal_only_by_default`
Gates (exact commands):
- `cargo xtask gate slice-9`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-9 --linux-vm` (deferred while VM_OFF; required before any acceptance)
Out of scope (recorded obligations, not fixed here):
- `crates/holden-cli` in any form (slice-10's lane).
- Harness configs, embeddings, packaging (slice-10); editing any pinned DONE list.
- xtask changes beyond the single SLICES registration entry.
Owner decision points:
- None.
