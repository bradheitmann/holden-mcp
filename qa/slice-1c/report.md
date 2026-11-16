# Independent QA report — slice-1c

**Verdict: accepted.** Frozen commit `9668c6897fea2d300b5f8de556f425ef92660395` passed all three slice proofs on a clean scratch clone. No critical or high defects found; no new defects recorded.

## Checklist

- [x] Bytes match the freeze record: all 97 population entries, their Git object identities and modes, the `6b640c5b5e9fdbd1c358ea95c8ce04481a0b424c` tree, and population SHA-256 `53c78aa47e6f63f7be8ee786a347677e82b983a14e42b401e349b2e2d188c31e` verified.
- [x] Host gate: `cargo xtask gate slice-1c` green on macOS/aarch64, 38/38 tests. Each invocation and gate step used isolated target directories under `/tmp/holden-mcp-qa1c-target-*`.
- [x] Linux reopen: 38/38 tests, identical names and counts to host; `green_with_declared_limitations` for absent cargo-deny, cargo-audit and charter tools, covered by host.
- [x] P1 fresh-clone guard reopen: green; guard explained the pin reason for the slice-1b QA files.
- [x] P1 unpinned QA mutant refused.
- [x] P1 one-byte pin mutation refused by filename; restored bytes and green guard.
- [x] P2 directory-wide exemption mutant made `new_decision_claiming_mit_is_refused` fail.
- [x] P2 new owner-decision claim refused.
- [x] P3 omission mutant made `licensor_exception_lists_every_file` fail.
- [x] Verifier mutant (accepting a non-accepted dispatch record) made the P1 pin test fail.
- [x] All mutants reverted; all three proof tests and final guard passed again.
- [x] `charter check --strict` passed. It emitted existing missing cost-row warnings for slice-0 and slice-1b.

## Findings

Slice-1b obligation D1 is closed by the narrowed owner-decision exemption. No new defects. The Linux result is qualified only by the declared host-covered tools above.

A preliminary CLI help probe compiled xtask in the scratch clone's default target directory. It did not run a gate; the complete host gate was rerun with an explicit outer `CARGO_TARGET_DIR` and per-step target directories, and the Linux gate used the same isolated-target discipline.

See [evidence.md](evidence.md) for commands, mutant outcomes, run-record paths and the Linux/host test-name comparison.

## Receipt validation

Recorded current reopen attestations for proofs 1–3 in `charter/.freeze/reopen/slice-1c.json`, each bound to the dispatched DONE hash, frozen manifest, and SHA-256 of `evidence.md`. Final validation passed: `charter receipt qa/slice-1c/receipt.json --verify-tree .` reported that the tree matched the recorded freeze.
