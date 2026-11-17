# Slice-18h read-only client cross-check — attempt 2

Candidate: `a53511f165b0ec3d8d8f1211d3fb0563e9177643`.

## Independent method

QA used the independent counter developed from P1-P7 before reading the adapter: outside code fences, count top-level `N.`/`Nb.` items in proof-labeled blocks; apply the title fallback only when labeled blocks have no item; otherwise apply the marked-list fallback only when it can yield no candidate. A short item requires an indented `-`/`*` sub-item; plain indented continuations never qualify. The title heading never opens a labeled block. Bullet items control fallback selection only; compared candidate counts are numbered-only. Final counter SHA-256: `2a6c8d83f2f91c73d3487c5ceafa95484d4f552a4768534cfa883d6c0f8f5e27`.

The repaired real `read_promises` adapter and `done_source_accounting` ran on Boat over the same exact `git archive` snapshots used in attempt 1. Every returned promise passed `verify_against`. QA compared every DONE file in both directions, including `seen` and `taken`, rather than comparing only totals. No client repository or host state was written.

## Commits and per-file results

| Snapshot | Exact commit | DONE files | Independent numbered taken / seen | Adapter all candidates | Adapter numbered taken / seen | Too short | Outside | Differing files |
|---|---|---:|---:|---:|---:|---:|---:|---|
| CLIENT-WAVE-0 finding | `5fb964bad3f27fd7ba29b0f83a78d22a969b2210` | 290 | 1493 / 1511 | 1622 | 1493 / 1511 | 1 | 17 | None |
| CLIENT-WAVE-0 current | `8a74fb06a7d5dd6e55976a3c2de5a2c9d2b3fd7f` | 311 | 1675 / 1738 | 1809 | 1675 / 1738 | 1 | 62 | None |
| CLIENT-ODIN current | `8c26fb36e7032c32def210be8a68ae3e6577b4a2` | 36 | 174 / 177 | 174 | 174 / 177 | 0 | 3 | None |
| CLIENT-PLANGRAPH current | `6373fce05c7a4d6d6e5b5c6d81f7e28caecf0ea5` | 42 | 283 / 290 | 283 | 283 / 290 | 0 | 7 | None |

Adapter runs exited 0: finding in 6s; current CLIENT-WAVE-0, CLIENT-ODIN and CLIENT-PLANGRAPH in 6s, 5s and 5s. All four normalized per-file comparisons have zero differing rows in either direction.

Attempt-1 D1 is closed by direct reopen: in both wave-0 snapshots, `charter/DONE/p00-depth-oracle-scope.md` is now independent **0 taken / 1 seen** and adapter **0 taken / 1 seen / 1 too_short / 0 outside**. Its five remaining adapter candidates are bullets, not the short numbered item. This exactly matches the independent rule and the new regression fixture.

## Required four files at the finding commit

| DONE file | Independent | Adapter | Required |
|---|---:|---:|---:|
| `charter/DONE/offline-revalidate.md` | 5 | 5 | 5 |
| `charter/DONE/calibration-recovery-route.md` | 6 | 6 | 6 |
| `charter/DONE/p05-source-capture.md` | 6 | 6 | 6 |
| `charter/DONE/reconciler-candidate-view.md` | 4 | 4 | 4 |

The required 5/6/6/4 verification passes with the repaired adapter.

## Numbered items left out at the finding commit

Columns are adapter `seen`, `taken`, `too_short`, and `outside any proof list`. Every file with `seen > taken` is listed.

| DONE file | Seen | Taken | Too short | Outside |
|---|---:|---:|---:|---:|
| charter/DONE/advisory-validator-depth.md | 5 | 0 | 0 | 5 |
| charter/DONE/fix-p00-seed.md | 5 | 4 | 0 | 1 |
| charter/DONE/human-knowledge-1.md | 17 | 11 | 0 | 6 |
| charter/DONE/p00-depth-oracle-scope.md | 1 | 0 | 1 | 0 |
| charter/DONE/p02-p04-output-split.md | 8 | 6 | 0 | 2 |
| charter/DONE/p05-c6-c9.md | 13 | 11 | 0 | 2 |
| charter/DONE/schema-repair.md | 7 | 6 | 0 | 1 |

## Numbered items left out: current CLIENT-WAVE-0

| DONE file | Seen | Taken | Too short | Outside |
|---|---:|---:|---:|---:|
| charter/DONE/advisory-validator-depth.md | 5 | 0 | 0 | 5 |
| charter/DONE/calibration-controls-current-scorer.md | 10 | 6 | 0 | 4 |
| charter/DONE/delta-reconciliation.md | 21 | 12 | 0 | 9 |
| charter/DONE/factory-06-verified-model-limits.md | 11 | 7 | 0 | 4 |
| charter/DONE/fit-slice-p00-p01.md | 21 | 18 | 0 | 3 |
| charter/DONE/fix-p00-seed.md | 5 | 4 | 0 | 1 |
| charter/DONE/human-knowledge-1.md | 17 | 11 | 0 | 6 |
| charter/DONE/p00-depth-oracle-scope.md | 1 | 0 | 1 | 0 |
| charter/DONE/p02-p04-output-split.md | 8 | 6 | 0 | 2 |
| charter/DONE/p02-supersession-retires-settled-concerns.md | 19 | 11 | 0 | 8 |
| charter/DONE/p05-c6-c9.md | 13 | 11 | 0 | 2 |
| charter/DONE/p05-failed-capture-null-access-date.md | 10 | 9 | 0 | 1 |
| charter/DONE/render-projections.md | 20 | 17 | 0 | 3 |
| charter/DONE/repo-pii-redaction.md | 19 | 9 | 0 | 10 |
| charter/DONE/schema-repair.md | 7 | 6 | 0 | 1 |
| charter/DONE/scorer-honours-failed-criteria.md | 8 | 5 | 0 | 3 |

## Numbered items left out: current CLIENT-ODIN

| DONE file | Seen | Taken | Too short | Outside |
|---|---:|---:|---:|---:|
| charter/DONE/charter-pin-cf05eb5.md | 8 | 5 | 0 | 3 |

## Numbered items left out: current CLIENT-PLANGRAPH

| DONE file | Seen | Taken | Too short | Outside |
|---|---:|---:|---:|---:|
| charter/DONE/gate-runner-v1-dev.md | 15 | 8 | 0 | 7 |
