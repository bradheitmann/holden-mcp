# Done list — slice-6 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-6 --dispatch`
Scope: completion criterion 11. `modules/holden-forge/` is a self-contained module with its own Cargo workspace, and no crate in the main workspace depends on it. It follows the S-FORGE pattern: `VERSION`, `UPSTREAM.md`, `SHA256SUMS`. It contains:
- the vendored domain-forge, synthesis and agent-forge methodology (the owner's proprietary work, D-06, licensed PolyForm Noncommercial);
- a holdout-engineering domain pack;
- holden agent constitutions for planner, sealer, evaluator and adversary;
- a Rust `forge` binary with `pack`, `constitution` and `export`.

The main workspace reads domain packs and constitutions only as files, through a typed loader in `holden-cognition`.
Proofs:
1. P1 isolation: no crate in the main workspace has a path or package dependency on anything under `modules/holden-forge/`. The forge workspace has no path dependency outside its own directory. A test checks both directions with `cargo metadata`.
   - Reopen: run the test.
   - Fail-without mutant: add a path dependency from `holden-cognition` to the forge crate. The test `forge_is_isolated` must fail.
   - Test: `xtask::tests::forge_is_isolated`
2. P2 export-standalone: `forge export <dir>` writes the module to an empty directory outside the repository and regenerates `SHA256SUMS`. It refuses (exit 1, naming the file) if any exported file contains an absolute path, a private identifier from the denylist, or a reference to the parent repository. The exported directory then builds and passes its own tests with no network and nothing from the parent repository.
   - Reopen: on both OSes, export to `/tmp`, run `cargo test --offline` there, and verify `SHA256SUMS`.
   - Fail-without mutant: put an absolute home path into one exported methodology file. The export must refuse.
   - Test: `holden-forge::tests::{export_refuses_absolute_path, export_builds_standalone}`, plus the reopen step
3. P3 domain-pack-and-constitutions: `forge pack` validates the holdout-engineering pack against the forge's typed schema. `forge constitution --role <role>` emits a constitution for each of the four roles. Every constitution section traces to a pack field, following the methodology's derivation map. The output is deterministic, and the forge.s own typed reader parses each emitted constitution and pack back without loss (round trip).
   - Reopen: run the commands and the round-trip test.
   - Fail-without mutant: drop one pack field from the derivation. The test `constitution_sections_trace_to_pack` must fail.
   - Test: `holden-forge::tests::{pack_validates, constitution_sections_trace_to_pack, deterministic_output}` and `holden-forge::tests::roundtrip_lossless`
4. P4 provenance-and-license: `UPSTREAM.md` lists the methodology source (by generic id) and every delta from it. Every forge file is covered by `SHA256SUMS`. The module's license is PolyForm Noncommercial 1.0.0, with the owner's Required Notice in the module and in every export.
   - Reopen: verify `SHA256SUMS` in the module and in an export.
   - Fail-without mutant: change one methodology file without updating `SHA256SUMS`. The verify step must fail.
   - Test: `holden-forge::tests::sums_cover_every_file`
Gates: same as slice-2, plus the export reopen.
Out of scope: the YAML and graph view generators from S-FORGE (later); forging domain packs for other domains (done in slice-7 for the embed target).
Owner decision points: none. D-06 is resolved: the forge is the owner's proprietary work, under PolyForm Noncommercial.
