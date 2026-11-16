# Owner decision — 2026-11-04 license-polyform-noncommercial

approved_by: owner

## Context

The owner approved the plan's defaults on 2026-11-04 (`2026-11-04-owner-approval-and-defaults.md`), which set D-07 to a permissive license. The same day the owner withdrew that default and stated the licensing intent for the repository and for the forge. D-06 (vendoring the S-FORGE methodology) was open pending a licensing confirmation. The slice-1b done list (`charter/DONE/slice-1b.md`) carries out this decision.

## Decision

Owner, 2026-11-04, verbatim: "I don't want this to be MIT. In other words, the Forge licensing is proprietary. I'm happy to allow people to use it, but I don't want people forking it or using my Forge stuff without paying me, I guess. Personal use only, I think, or non-commercial: they can use it, but they can't sell it."

Owner, 2026-11-04, verbatim: "Go ahead and create a repo for this on Bradley Heitmann's GitHub."

- D-07 (replaces the earlier default): the repository is licensed under PolyForm Noncommercial 1.0.0 (SPDX `PolyForm-Noncommercial-1.0.0`).
  - `LICENSE` is byte-identical to `PolyForm-Noncommercial-1.0.0.md` from the PolyForm project at upstream commit `76a278c402bc43b8d2b561da140b0f3e17263015` (sha256 `c0ea4a896d2c8c394b29f9427589996db826cd501c512279ff0ed3ef48fabbe5`).
  - `NOTICE` holds the two Required Notice lines from the done list: the copyright line naming the licensor, and the commercial-use line pointing to the repository's issue tracker. The owner may replace the contact with an email address later.
  - Every workspace crate declares `license = "PolyForm-Noncommercial-1.0.0"`. `cargo deny` allows that license for the workspace crates only; the third-party allow list is unchanged.
  - The licensor name is the name in the second quote: the owner chose to hold the repository under that name.
- D-06 (resolved): S-FORGE is the owner's proprietary work ("the Forge licensing is proprietary"). It is vendored into this repository, and the forge module (slice 6) carries the same license, PolyForm Noncommercial 1.0.0. No separate upstream licensing confirmation is needed.

## Licensor-name exception to the common engineering standard §1

- What: the standard asks committed text to use generic ids, with no private names. A license notice must name its licensor, so the licensor name is committed in exactly these files, each listed with its reason in `charter/safety-allow.txt`:
  - `NOTICE` (the Required Notice lines);
  - `charter/owner-decisions/2026-11-04-license-polyform-noncommercial.md` (this record: the owner's verbatim instruction that names the licensor);
  - `charter/DONE/slice-1b.md` (the done list that states the Required Notice lines; pinned at dispatch);
  - `modules/holden-forge/NOTICE` (forthcoming with slice 6: the forge module's copy of the Required Notice lines).
- A test (`licensor_exception_lists_every_file`) compares this list with the tracked files that carry the licensor name. A file marked forthcoming may be absent from the branch under test. (Format change approved by the orchestrator on 2026-11-04, slice-1c: explicit paths, forthcoming marker. The decision itself is unchanged.)
- Reason: a copyright notice without the holder's name does not identify who licenses the work, so a generic id cannot stand in for it.
- Owner: the owner.
- Expiry: none.
- The guard pins `NOTICE` by sha256, so the guard source does not carry the name.

## Rejected alternative

- The permissive default adopted earlier: the owner refused it in the first quote.
- A proprietary all-rights-reserved notice: the owner wants people to be able to use the work noncommercially.

## Follow-ups

- Commits made before this decision carried the earlier license text. They were never published (the repository is private). Rewriting that history is out of scope and would be a separate owner decision.
