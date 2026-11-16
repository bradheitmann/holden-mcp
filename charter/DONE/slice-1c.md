# Done list — slice-1c (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-1c --dispatch`
Scope: make `main` pass its own guard again, and close obligation D1 from slice-1b. The guard's `stale_license_mentions` check exempts immutable, hash-pinned verification history. It stops exempting owner-decision records wholesale. No feature work.
Proofs:
1. P1 qa-records-exempt-by-pin: files under `qa/<slice>/` are exempt from `stale_license_mentions` only when that slice is recorded accepted and the file's current SHA-256 is listed in `charter/qa-pins/<slice>.sha256`. That pin record is written by the orchestrator at acceptance, for slice-0 and slice-1b in this slice. Any other file under `qa/`, or a pinned file whose bytes changed, is still checked.
   - Reopen: on a fresh clone of the candidate, `cargo xtask guard` passes. The previously refused `qa/slice-1b/*` lines are exempt and the guard names the reason.
   - Fail-without mutant: add `qa/slice-x/notes.md` containing "This repository is MIT licensed". The guard must refuse it.
   - Test: `xtask::tests::{qa_record_exempt_only_when_pinned, unpinned_qa_file_is_checked}`
2. P2 owner-decisions-narrowed (closes D1): `charter/owner-decisions/` is no longer exempt as a directory. Only files listed in `charter/license-mention-allow.txt` are exempt, each with a one-line reason. A new owner-decision file stating the repository is MIT-licensed is refused.
   - Reopen: run the guard. Add the probe file in a scratch clone and observe the refusal.
   - Fail-without mutant: restore the directory-wide exemption. The test `new_decision_claiming_mit_is_refused` must fail.
   - Test: `xtask::tests::new_decision_claiming_mit_is_refused`
3. P3 licensor-exception-complete: the licensor-name exception record lists every tracked file that carries the owner's name, and each one is in `charter/safety-allow.txt`. A test compares the record's list with the files matching the name hash.
   - Reopen: run the test.
   - Fail-without mutant: remove one file from the exception record. The test `licensor_exception_lists_every_file` must fail.
   - Test: `xtask::tests::licensor_exception_lists_every_file`
Gates: fmt, clippy, guard, `cargo xtask gate slice-1c`, the VM gate, and `charter check --strict`.
Out of scope: any change to the license text, the canon, or features.
Owner decision points: none.
