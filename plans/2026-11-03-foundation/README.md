# Foundation dispatch graph (slices 0-7)

A retrospective reformat of the holden-mcp foundation slice plans (0, 1, 1b, 1c, 2, 3, 4, 5, 6, 7)
into the plangraph-shaped DEV/QA dispatch graph used by `plans/2026-11-06-integration/`.
Nothing was planned in this shape before dispatch.

- The charter DONE lists (`charter/DONE/slice-*.md`; slice-7 on branch `slice-7`) stay normative.
  Proofs here are shortened to one line each; if they differ, the DONE list wins.
- Status, QA attempts and repair cycles come from `charter/STATE.md`, `qa/slice-*/receipt.json`,
  `charter/qa-pins/` and `git log main`. Slices 0-6 are ACCEPTED; slice-7 is DEV_COMPLETE_QA_PENDING.
- Dependency choices (including slice-6 placement) are explained in `format_note`.

Validate from the repository root (the result must show `"valid": true`):

    jq -f plans/2026-11-03-foundation/validate-graph.jq plans/2026-11-03-foundation/dispatch-graph.json
