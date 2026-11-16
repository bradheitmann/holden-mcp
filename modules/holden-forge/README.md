# holden-forge

A self-contained domain forge. It holds:

- `methodology/`: the vendored domain-forge, synthesis and agent-forge methodology (see `UPSTREAM.md`);
- `packs/holdout-engineering/PACK.json`: a holdout-engineering domain pack in the forge's typed schema;
- `agents/holdout-engineering/<role>.md`: agent constitutions for the planner, sealer, evaluator and adversary, emitted from that pack;
- `src/`: the Rust `forge` binary and its library.

The module is its own Cargo workspace with no third-party crate, so it builds and tests offline with only the Rust toolchain pinned in `rust-toolchain.toml`.

## Commands

Run from this directory with `cargo run --offline --quiet -- <command>`:

| Command | What it does | Exit codes |
|---|---|---|
| `pack [--pack <file>]` | Reads the pack with the typed reader, validates it, and checks it is in canonical layout. Prints counts and the pack digest. | 0 valid, 1 refused (every error named by JSON pointer) |
| `constitution --role <role> [--pack <file>]` | Derives the constitution of `planner`, `sealer`, `evaluator` or `adversary` and prints it. | 0, 1 |
| `export <dir> [--denylist <file>]` | Writes the module into an empty directory outside the repository that holds it, regenerates `SHA256SUMS` there and verifies it. Refuses first, naming the file and line, if any file holds an absolute path, a private identifier from the denylist, or a reference to the parent repository. The denylist comes from `--denylist`, else `$FORGE_DENYLIST`. | 0, 1 refused, 3 no denylist (never a pass) |
| `verify [<dir>]` | Checks that `SHA256SUMS` covers every file with a matching hash, and that `LICENSE` and `NOTICE` are the pinned texts. | 0, 1 |
| `sums [<dir>]` | Regenerates `SHA256SUMS`. | 0, 1 |

`SHA256SUMS` uses the `shasum -a 256 -c` format. It covers every file except itself, the `target/` build directory, `.git/` and `.DS_Store`.

## How a constitution is derived

`methodology/LW-AGENT-FORGE.md` §2 maps pack fields to nine constitution sections. The forge implements that map:

| Section | Pack fields |
|---|---|
| IDENTITY & BINDING | `role_focus[].identity`, `pack_name`, `domain`, `ratification_status`, `priority_order[]` |
| DOMAIN MAP | every `super_domains[].name`, `categories[].name` and `categories[].seam`; super-domains outside the role are marked `not-yours` |
| ROLE LENSES | the `worldviews[]` the role cut names |
| OPERATING RULES | `synthesis_instructions[]` of the items in the role's super-domains |
| SELF-QC PROTOCOL | `internal_evaluation_criteria[]` (question, pass condition, checklist) of those items |
| TASK ROUTING | `applicability_signals[]` and `anti_signals[]` of those items |
| FAILURE-MODE REGISTRY | `signature_failures[]` of those items |
| CROSS-CUTTING LAWS | `cross_cutting_postures[]` |
| EVIDENCE DISCIPLINE | `module_laws[]` |

Every entry ends with `[trace: <JSON pointer>]` naming the pack field it came from, and item-derived entries cite the item as `CC.II`. Each constitution states the pack digest (`sha256` of the pack file); when the pack changes, re-emit the constitutions. The test `deterministic_output` fails while a committed constitution is stale.

The pack schema follows the CCII depth contract of `methodology/LW-DOMAIN-FORGE.md` §0 in one JSON file. Two typed fields stand in for things the methodology leaves in prose or in other files: `signature_failures[]` (the methodology names signature failures inside `detailed_description`) and `role_focus[]` (the role cut of LW-AGENT-FORGE §7).

## License

PolyForm Noncommercial 1.0.0 (`LICENSE`), with the Required Notice in `NOTICE`. Every export carries both.
