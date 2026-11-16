# slice-1 QA report — attempt 3

**Verdict: accepted.** This is the third and final QA attempt, released by the owner under S5. The judged bytes are frozen commit `cc677f6f4febc4ffaffc378aac2aea51a796be89`.

## Results

- All 81 entries in `charter/.freeze/slice-1.json` matched the frozen tree by path, Git object, mode, type, and SHA-256. The manifest digest matched `43454d6d223a8637856121b193ca932d30e5f5ea9b43e3e5c1ee218dbdfe5037`; the freeze record has no patch hash (`null`).
- The required toolchain was Cargo 1.95.0 (`f2d3ce0bd`, 2026-03-21).
- A clean macOS/aarch64 scratch clone passed `cargo xtask gate slice-1`: formatting, Clippy, 48 tests, guard, cargo-deny, cargo-audit, and Charter safety all passed.
- The Linux/aarch64 VM reopen passed formatting, Clippy, and the same 48 tests with identical test names and outcomes. The VM reported `green_with_declared_limitations` because cargo-deny, cargo-audit, and Charter are not installed there; the host guard covered those checks. The target-directory base was honored.
- `charter check --strict` returned `charter: ok`, with only the existing slice-0 cost-row warning.
- All six required fail-without mutants and an independent P3 mutant produced red results. Each mutation was discarded. A final clean-worktree full test run passed all 48 tests.

## Proofs reopened

P1 fixed vector and seal round trip; P2 ciphertext, AAD, nonce, and wrong-key refusal; P3 server-secret dependence and foreign-key proof refusal; P4 nonce commitment; P5 file-key-store protections; and P6 VM address override and target-base behavior. The host and VM both passed `seal_fixed_vector`, which asserts the same `plan_commit` bytes: `2d368c47d0cdb02515b2225e1509a885844309a504f6c2109b5e525f014b8bb5`.

No critical or high defect was found. No code changes were made to the frozen candidate.
