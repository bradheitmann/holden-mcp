# Independent QA report — slice-2 — attempt 2

**Verdict: accepted.** The frozen bytes were unchanged. The fresh host and Linux gates passed all 71 tests with matching names and counts. The slice-1 prerequisite is recorded accepted on main and its receipt validates against its original frozen candidate. No slice-2 implementation defect was found.

## Checklist

- [x] Frozen bytes verified: 155 manifest entries, exact head, tree, and manifest SHA-256 matched.
- [x] Host gate green: macOS/aarch64, Cargo 1.95.0, 71 passed, 0 failed, 0 ignored.
- [x] Linux gate green: Linux/aarch64, 71 passed, 0 failed, 0 ignored; test names and counts match the host. The Linux P2 peer-credential tests passed. VM guard limitations for absent `cargo-deny`, `cargo-audit`, and `charter` were covered by the complete passing host guard.
- [x] P1–P6 fail-without mutants: executed (attempt 1), per checks C08–C13 in the archived attempt-1 receipt; each named regression test failed while mutated and passed after revert.
- [x] Verifier P2 role-enforcement mutant: executed (attempt 1) per C14; additionally repeated in attempt 2, where `implementer_refused_reveal_params` failed under mutation and passed after restoration.
- [x] Verifier P5 secret-egress mutant: executed (attempt 1) per C15.
- [x] Real transport review: tests use an `rmcp` client over a real Unix socket and real stdio process. No mock of the system under test was found.
- [x] `charter check --strict` passed, with existing missing cost-row warnings for accepted slices 0, 1b, 1c, and 6.
- [x] Acceptance prerequisite: main at `0023b36` records slice-1 accepted, the owner's S5 decision approves the final QA attempt, and the slice-1 receipt says accepted. The receipt validator refused when run against descendant `0023b36` because the later tree has a `Cargo.lock` difference outside the receipt's freeze; it passed when verified against its original frozen candidate with the authoritative freeze/reopen/pin records from main.

## Evidence-integrity note

Attempt-1 receipt checks list the required gates and mutants as executed. Its archived report and evidence files have been hashed for this attempt, but the receipt contains no corresponding expected hashes, so those archive bytes cannot be authenticated against the receipt. The attempt-1 records are marked “executed (attempt 1)” as requested; attempt 2 independently reran the complete host and Linux gates and a P2 role-enforcement mutant. This is recorded as informational obligation D1, not a product defect.

## Findings

No slice-2 code defect was found. D1 concerns the missing hash binding in the prior QA record only. No secret values or absolute home paths are included in this report.
