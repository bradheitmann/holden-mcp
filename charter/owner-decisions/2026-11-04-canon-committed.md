# Owner decision — 2026-11-04 canon-committed

approved_by: owner

## Context

D-08 asked whether to commit the four S-DOCTRINE canon files or to load them from the private root with only the pin committed. Until now the default was the private root, so a fresh clone could not verify the canon and returned `canon_absent`.

## Decision

Owner, 2026-11-04, verbatim: "You can commit the cannon files if they are part of the codebase. It's completely up to you: are they referenced or used in Holden-MCP?"

They are referenced and used: `canon/CANON-PIN.txt` pins them, `holden canon verify` checks them, and G-SITUATED derives cognitions and holdouts from the canon. So, under the owner's condition:

- D-08 (resolved): the four canon files are committed under `canon/holden/` with their base names:
  - `holdout_adapter_protocol_v0_2_0.md`
  - `holdout_skill_metaprompt_v2_1_1.md`
  - `holdout_skill_schema_registry_v2_3_0.yaml`
  - `holdout_skill_spec_v2_3_0.md`
- Each file is byte-identical to `canon/CANON-PIN.txt` (canon version `skill-v2.3.0 adapter-v0.2.0`), which matches the hosted S-CANON-MCP pin. The test `holden-canon::tests::committed_canon_matches_pin` proves it.
- `holden canon verify` defaults to `--root canon/holden --pin canon/CANON-PIN.txt`. A private-root copy can still be verified with `--root <dir>`.
- Before commit, the files were scanned for private names, absolute home paths and secrets by `cargo xtask guard` and `charter safety`, and none were found. They are the owner's own work and are covered by the repository license (`2026-11-04-license-polyform-noncommercial.md`).

## Rejected alternative

- Keeping the files only in the private root: a fresh clone could not verify the canon, and the owner left the choice to whether the files are used here. They are.

## Follow-ups

- None.
