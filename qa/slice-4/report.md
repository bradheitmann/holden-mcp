# Slice-4 independent verification report — attempt 2

**Verdict: accepted.** The frozen candidate now passes the complete macOS host gate from a clean scratch clone with 111/111 tests, matching the Linux gate count. The host confinement path is mutation-verified: bypassing confinement makes `escape_attempt_is_blocked` fail, and restoring the frozen implementation makes it pass. The earlier attempt's executed evidence supplies the remaining Linux, mutant, fixture and reopen results.

## Checklist

- [x] Frozen bytes match the freeze manifest (attempt 1 executed; archived receipt and evidence confirmed present).
- [x] Prerequisites verified: slices 1, 2 and 3 each report “tree matches the recorded freeze.”
- [x] Host gate green: 111 passed, 0 failed.
- [x] Linux gate green: attempt 1 reports 111 passed, 0 failed; the host count is now also 111.
- [x] M1–M6: each named test turned red under mutation and green after revert in attempt 1; M4 also turned red and green on the macOS host in attempt 2.
- [x] Independent P3 mutant turned `tampered_record_is_refused` red and green in attempt 1.
- [x] Repair judged. D2 remains a medium obligation accepted by the orchestrator for a later slice; it does not block acceptance.
- [x] Fixture product is source-built from the evaluated commit and observed at its process/file boundary; no mocks or stubs were found.

## Defects and obligations

No critical or high defects. D1 from attempt 1 is resolved by the clean host run and host M4 red/green result in attempt 2.

D2 is an accepted medium obligation: the build-child leak watch no longer detects forwarding of cargo-derived `SSL_CERT_DIR` and `SSL_CERT_FILE`. The product's exact environment allow-list and runtime environment clearing remain in place.

Receipt validation: `charter receipt qa/slice-4/receipt.json --verify-tree .` exited 0 and reported that the checkout tree matches the recorded slice-4 freeze at `047a0000153b857a38d6e589051501c810b1a641`, manifest `fa62c79cf2d59c244eb15a155d9c87866f8235be6194add862fe1d6eee2d5f9d`.
