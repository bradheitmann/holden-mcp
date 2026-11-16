# Slice 5 verification evidence — attempt 2

## Frozen bytes and toolchain

- Frozen HEAD: `11f046b88e1e993d21feda43c7a3baf7aaada55c`.
- Manifest SHA-256: `b8e74f006795a36b26d4195d447943530179b6509c827412a6b05b5bd9920bb3`; entry count 252.
- Attempt-1 archive records verification of 252/252 per-file SHA-256 values and Git blob identities. Attempt 2 used clean detached clones at the frozen HEAD; the host gate reported tree clean.
- Binary patch from dispatch base `51a695f9ae106e1e5f6401489111f542674c6a14`: SHA-256 `7d3fd3b5532e5f7bfa74ca087f68ab6287c1c701956065380e404a78ccc93eb5`.
- `cargo --version`: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`.
- Scratch private root: `/tmp/holden-mcp-qa5b-root/holden-mcp/`, initialized with the supplied denylist and corpora. Gate and self-test target directories were under `/tmp`.

## Prerequisites

`charter receipt ... --verify-tree` succeeded for slices 1, 2, 3, and 4. Frozen heads were `cc677f6f4febc4ffaffc378aac2aea51a796be89`, `c4d998e5ab8315c5aec5a3ec82bd71d9d9454796`, `99a5bbfe14c935f470acec8bd6adac78f058bc20`, and `047a0000153b857a38d6e589051501c810b1a641`. The main checkout at `3c11179ff80cbc3b6af7bc4fc2943fbc42ecd151` was not modified. Its object database did not contain the slice-4 frozen commit, so that exact commit was checked from a clean clone sourced from this checkout against the read-only main receipt and freeze metadata.

## Gates and isolation

- Host command: `cargo xtask gate slice-5`, clean scratch clone, isolated private root, own Cargo target directory. Result `green`; 126 passed, 0 failed, 0 ignored. fmt, clippy, self-test, guard, 36 slice-4 replays, and all 11 ledger verifications passed. Self-test catch count was 1/1 for each of its five defect classes.
- Required Linux wrapper: `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-5 --linux-vm`. Result `green_with_declared_limitations`; 126 passed, 0 failed, 0 ignored; wrapper reported test names identical and counts identical (host 126 / VM 126). Guest guard limitations were `cargo_deny_absent`, `cargo_audit_absent`, and `charter_absent`; the host guard passed those lanes.
- The wrapper implementation does not forward `AGENT_PRIVATE_ROOT` to its guest invocation. I therefore copied the supplied denylist and corpora into a dedicated guest `/tmp` root and reran the complete gate directly from a clean frozen clone with `AGENT_PRIVATE_ROOT=/tmp/holden-mcp-qa5b-root`, `CARGO_TARGET_DIR=/tmp/holden-mcp-qa5b-vm-xtask-target`, and `HOLDEN_TARGET_BASE=/tmp/holden-mcp-qa5b-vm-target-base`. Result `green_with_declared_limitations`; 126 passed; self-test green with all five catches 1/1; 36 replays and 11 ledger verifications passed.
- Real-root database custody: sorted path/mtime/size snapshots from before and after all host-side work compared equal. Ledger contents were not opened. The wrapper’s guest default-root write was on the separate VM, not on the host real root; the isolated guest rerun was performed afterward.

## Mutants and G-MEASURED substance

All named mutants and the independent mutant were executed in attempt 1 and are classified `executed (attempt 1)` in this receipt's descriptions. The archived attempt-1 report/evidence contains their red and restored-green output:

- M1 previous-hash link removed: `tampered_history_is_detected` failed; restored test passed.
- M2 presumed catches without evaluator execution: `catch_rate_from_real_runs` failed; restored test passed.
- M3 point estimate substituted for the bound: `authority_uses_pessimistic_bound` failed; restored test passed.
- M4 ledger weights ignored: `draw_uses_ledger_weights` failed; restored test passed.
- M5 one never-again test disabled: that defect's self-test catch count dropped to zero and the gate failed; restored run caught all five.
- M6 confinement violation omitted from decision: `confinement_violation_fails_verdict` failed; restored test passed.
- M7 independent G-MEASURED mutant dropped control rows from measurement reconstruction: `per_lens_and_cognition_rates` failed; restored test passed.

Attempt-1 executed evidence confirms `run_controls` sends control variants through the real fixture evaluator and derives measurements from signed verdict records. Wilson lower catch and upper false-fail bounds determine authority; unmeasured or undersampled checks remain advisory. Two ledger-feedback rounds changed weights and the next keyed draw; observed weights were `2,4,6,4,7` then `2,5,8,4,6`, with minimum weight 1.

## Informational note

Attempt 1's host self-test appended five verification records to the real append-only ledger. That fact is noted as QA history only, not as a defect in slice 5. Attempt 2 did not inspect ledger contents or modify the real `db/` directory.
