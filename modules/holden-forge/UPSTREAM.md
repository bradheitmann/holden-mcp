# UPSTREAM: provenance and deltas

## Source

- **Id:** S-FORGE, a domain-forge module in another of the owner's projects. The map from this id to the real source is private and is not part of this module.
- **Version read:** `0.4.0` (upstream build metadata: the owner's methodology repository at source head `52228165`).
- **Read on:** 2026-11-04.
- **License:** the owner's proprietary work, licensed here under PolyForm Noncommercial 1.0.0 by owner decision (D-06). See `LICENSE` and `NOTICE`.

## Vendored files

Each file below is copied from the source's `synthdoc/` directory into `methodology/` under the same name. "Source sha256" is the upstream file; "vendored sha256" is the file in this module.

| File | Source sha256 | Vendored sha256 | Delta |
|---|---|---|---|
| `BOOT-CONTRACT.md` | `31de6b5272b8a8e8fac5a1df837d8c759125dd0e3139379e61179bcbb2394b3b` | same | none (byte-for-byte) |
| `DOMAIN-INPUT-MANIFEST.template.yaml` | `dd6a4f06f3001ebb34366adf39deaf74dec0ed2f7c4e0b40fa6c8cfe729afad3` | same | none (byte-for-byte) |
| `RUN-MANIFEST.template.yaml` | `8498dadbc3e95da26865f46624e17663e5781dbd98d5f05bcfa5617db79f6d37` | same | none (byte-for-byte) |
| `LW-AGENT-FORGE.md` | `5e048fb0310c6719ed53c1e0e1f3a6e04db43ccf92700458f5bd03dcaea46290` | `30fc7c3f8be0164a57fde166e838bd20502133f057fef5287631ee6a43bfcb35` | D1 |
| `LW-DOMAIN-FORGE.md` | `1ca8ac35ec8327066e4af405df023d22c5bdc3c8c62196546c93435fafe77fc4` | `e3c795919dae4f5f7feac53c9f9797845b86c0015b6b345af5322251a1c243a8` | D2 |
| `LW-SYNTHESIS-PRIME.md` | `9a257e5fb9322c3e7d9e0f32c3760d4a4985a5b4680584687f3f43fc6f6aaf8f` | `0f0f828e685fd3143933182a2aaacdc10abb3c6cc87a51bbd3718a66915d1c52` | D3 |
| `TEAM-TOPOLOGY.md` | `df13ee40d3f1b456b1d1b8b7eb88c7f8dbbbb83fbfb45fdb9555ec72b134a7b8` | `dca2d04875762bfc387d5c6b3703878064f51f1f873dab22abc396046fb67192` | D4 |
| `synthdoc-output-schema.md` | `dd67c10ccf61c159265bad231e5de66480e7febebbd7ba436e90e686d9119ae0` | `9d062d6480ecd40c940cee8c88d1efa18a3a00109888d1e499e089b3794b491d` | D5 |

## Deltas

Every delta replaces a private project name, a private product name or a machine-local path with generic wording. No rule, gate, table or procedure changed meaning, except where D4 notes a renamed field.

- **D1 `LW-AGENT-FORGE.md`, line 14 (Lineage):** a private reference-prompt name and its machine-local directory became "RAC v4.5 (an owner reference prompt, kept outside this module, ~1 year old)".
- **D2 `LW-DOMAIN-FORGE.md`, lines 10–13 (Lineage):** a private company name became "the owner's"; a private reference-prompt name and its machine-local file path became "RAC v4.5 (an owner-provided reference prompt, kept outside this module)".
- **D3 `LW-SYNTHESIS-PRIME.md`, lines 1, 5, 6 and 9:** the upstream project's name was removed from the title, from "the of-record database" and from "the canonical home"; a private company and product name in the Lineage line became "an earlier owner synthesis prompt".
- **D4 `TEAM-TOPOLOGY.md`:**
  - lines 5 and 77: two private governance-skill names became "governance-protocol skills";
  - lines 35 and 41: the receipt field `<private name>_verify=YES` became `qa_verify=YES` (the field the QA role produces; the upstream gate's field name carries a private name);
  - line 79: the environment variable `<UPSTREAM>_OF_RECORD_HOME` became `OF_RECORD_HOME`;
  - line 81: the upstream project's name became "the upstream home";
  - line 86: the upstream pipeline binary's name became `<upstream>-pipeline`.
- **D5 `synthdoc-output-schema.md`, line 1 (title):** the upstream project's name was removed.

## Not vendored

- `synthdoc/schema/` (the 77-category agentic-engineering exemplar pack) and `synthdoc/agents/` (its exemplar constitution and skill): they forge a different domain. This module carries its own pack (`packs/holdout-engineering/`) and its own constitutions (`agents/holdout-engineering/`).
- `synthdoc/DOMAIN-SCHEMA-CATEGORY-SET.md`: the category index of that exemplar pack.
- `synthdoc/GRAPH-INTELLIGIBILITY.md`, the source's `views/` generator crate, `VIEWS.md`: the YAML and graph view generators are out of scope for this module.
- The source's `export.sh`: its behaviour is ported to Rust as `forge export` and `forge verify` (no shell script is shipped). The port checks every exported line for absolute paths anywhere in the line (the script checked line starts only), adds the private-identifier and parent-repository checks, refuses symbolic links, and writes a directory instead of a zip.
- The source's `README.md`, `CHANGELOG.md`, `SOURCE-HEAD.txt` and `SHA256SUMS.txt`: replaced by this file, `VERSION`, `README.md` and `SHA256SUMS`.

## New in this module (not in the source)

`Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `.gitignore`, `src/` (the `forge` binary and library), `packs/holdout-engineering/PACK.json`, `agents/holdout-engineering/*.md`, `README.md`, `UPSTREAM.md`, `VERSION`, `LICENSE`, `NOTICE`, `SHA256SUMS`.
