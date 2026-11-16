# Slice 5 independent verification report — attempt 2

**Verdict: accepted.** The frozen slice-5 tree passed both required gates with 126/126 tests. The host and required Linux wrapper reported identical test names and pass counts. An additional direct Linux run used an isolated private root after inspection showed that the wrapper does not forward `AGENT_PRIVATE_ROOT` into its guest command.

## Checklist

| Item | Result | Evidence |
|---|---|---|
| Frozen bytes | Pass | 252 manifest entries and blob identities matched; manifest SHA-256 and dispatch patch SHA-256 are recorded in the receipt and evidence. |
| Prerequisites | Pass | Slices 1–4 receipt/tree checks passed against their frozen heads. The slice-4 commit object was absent from the read-only main checkout, so its exact frozen commit was cloned from this checkout and checked against the main freeze record. |
| Host gate | Pass | Clean frozen clone; isolated private root; 126 passed, zero failed; self-test green, five defect catches; 36 replay records and 11 ledgers verified. |
| Linux wrapper gate | Pass | Required `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-5 --linux-vm`; 126 passed; wrapper reports identical test names and counts. |
| Linux private-root isolation | Pass on direct rerun | The wrapper does not forward the environment override. Re-ran the frozen Linux gate with the private root and targets under `/tmp`; 126 passed, self-test green, five catches, replays and ledger verification green. |
| M1–M6 | Pass | Executed in attempt 1; each named mutant went red and returned green after revert. |
| G-MEASURED and feedback | Pass | Attempt-1 executed evidence: real evaluator runs, Wilson pessimistic bounds, advisory-only under-sampling, two rounds of ledger-derived keyed draws, all weights positive. |
| Independent mutant | Pass | M7 in attempt 1: dropping control entries from measurement reconstruction broke the per-lens/cognition test; revert restored green. |

## Informational record

Attempt 1's self-test appended five verification records to the real append-only ledger. This is recorded as information about the earlier QA process, not as a slice defect. For attempt 2, metadata-only snapshots of the real `db/` file list, mtimes, and sizes were identical before and after; ledger contents were not opened. All attempt-2 private-root writes were directed to temporary storage.

## Environment limitation and correction

The required Linux wrapper passed and compared its test names and counts with the host run. Inspection of the frozen wrapper source showed that it forwards the build target base but not `AGENT_PRIVATE_ROOT`; its first guest run therefore used the guest's default data root. To meet the isolated-root requirement, I copied the supplied denylist and corpus to a `/tmp` private root on the guest and ran the same full gate directly from a clean clone of the frozen commit with that root and all Cargo targets under `/tmp`. This isolated run passed with the same 126 tests, self-test, ledger verification, and replay gates. The guest guard reported missing cargo-deny, cargo-audit, and charter tools as declared limitations covered by the host guard.
