# Slice-18g read-only client cross-check

Candidate: `8d9f96e1a01363562a5e38976c6b1473f6be1a37`.

## Method

Immediately before scanning, QA read each client's `HEAD` and confirmed that it had no tracked working-tree modifications. QA archived the committed `charter/INTENT.md`, `charter/GUARANTEES.yaml`, and `charter/DONE/*.md` inputs from those exact commits, staged the archives on the remote Linux worker, and ran the real `holden_cognition::read_promises` adapter over the staged snapshots (whose selected-document rules exclude templates). Every returned promise passed `verify_against` against its own source bytes. No client repository or host state was written.

The independent count used a separately written awk state counter, not adapter output or adapter code. It counted guarantee statements and whole top-level `N.`/`Na.` and bullet items under proof labels/headings; preserved blocks across prose, fences, tables, and lower-level subheadings as specified; recognized unlabeled title lists; excluded indented sub-items and short candidates; and applied the same post-item label termination and decoy exclusions. QA normalized the selected path sets and ordering, then compared counts file by file.

Cognition counts are the actual `.holden/cognitions/*.json` file totals in each host's existing worktree. Each host's own `promise-proposals.json` says `AWAITING_OWNER_APPROVAL`, and its persisted `candidate_promises` total equals its cognition-file total. These are earlier proposal waves, not an expected live recount: write-back is proposal-only by default, and QA did not re-embed or approve anything.

## Results

| Generic client | Exact commit counted | Adapter candidates | Independent count | Existing cognition files | Differing files |
|---|---|---:|---:|---:|---|
| CLIENT-WAVE-0 | `428dc85b6d40d6de08902e146bc6659009e755cb` | 1716 | 1716 | 879 | None |
| CLIENT-ODIN | `633aef4c40c3f20ad15f6eb856ee9a06da82736c` | 170 | 170 | 108 | None |
| CLIENT-PLANGRAPH | `5fe216336b77d082d8421f8a71757f0abac4ae1d` | 272 | 272 | 269 | None |

## Candidate/cognition explanations

- **CLIENT-WAVE-0:** the host proposal persists 879 candidates and 879 cognition files at host snapshot commit `5fb964bad3f27fd7ba29b0f83a78d22a969b2210`, still awaiting owner approval. The current source commit is different and the current adapter includes the repaired proof forms, so 1716 current candidates do not replace or invalidate that earlier proposal inventory. The same snapshot explains the 18f historical comparison of 978 adapter candidates versus 879 cognition files; the present 1716/879 comparison is a current-source recount versus that unchanged earlier wave.
- **CLIENT-ODIN:** the current source commit equals the host snapshot commit. Its persisted proposal has 108 candidates and 108 cognition files, awaiting owner approval. The new 170-candidate recount therefore measures the 18g extraction repairs on the same planning bytes; it is not a claim that the earlier proposal wave already wrote 170 cognitions.
- **CLIENT-PLANGRAPH:** the host proposal persists 269 candidates and 269 cognition files at host snapshot commit `370c5125fa82cdf5f90cee80b08b460d6eabd25d`, awaiting owner approval. The current count is 272 after the lettered-number repair. Most importantly, `charter/DONE/cs-private-root-v1-qa.md` is now **7 adapter / 7 independent**, closing the 18f **4/7** gap: `4b.`, `4c.`, and the following `5.` are all present.

## Per-file differing list

**None for all three clients.** The final normalized adapter and independent per-file count streams were byte-identical. In particular, the formerly differing CLIENT-PLANGRAPH file is no longer different.
