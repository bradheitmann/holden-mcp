{
  "schema_version": "charter.receipt/1",
  "slice": "slice-15",
  "verifies": "slice-15 independent adversarial QA: frozen identity and exact dependency-only scope, RUSTSEC-2026-0462 guard and fail-without mutant, pre/post-bump host and Linux name identity, real-repository and workflows regressions, and canonical charter strict validation",
  "candidate_commit": "7ad31eb693087438c82aeeec92925cc70a5bf0ac",
  "frozen": {
    "manifest_sha256": "1dc932e5b25f416914be2e6d4398007457c0109faa49b5209f38a1fc1282c763",
    "head": "7ad31eb693087438c82aeeec92925cc70a5bf0ac"
  },
  "dispatch": {
    "base_commit": "644756bdaf1d43a9f0727b9a98b0525a6daf40eb",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at 7ad31eb693087438c82aeeec92925cc70a5bf0ac with tree 184de5c0912da29f049960249d68924801a8c12d. The independently reopened charter.freeze/2 names that commit/tree, manifest 1dc932e5b25f416914be2e6d4398007457c0109faa49b5209f38a1fc1282c763 and 363 entries. Every listed path's Git object id, mode, type and file SHA-256 was compared: entries_compared=363, mismatches=0, comparison exit 0. DONE SHA-256 and the current dispatch done_sha256 both equal 833884e09c33e7d2042ae948faee6c0d65f31b4ffd6f48d95d117f99fed7ccba; dispatch base matches this receipt.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "SCOPE PASS. The complete 6af6628baf676b3ccfe52895c0e75967e3cf15b4..7ad31eb693087438c82aeeec92925cc70a5bf0ac diff touches exactly Cargo.toml, Cargo.lock, charter/DONE/slice-15.md and xtask/src/gate.rs, with no crate source or test adaptation. Cargo.toml changes only rmcp =3.8.0 to =3.9.0; Cargo.lock selects rmcp 3.9.0. The xtask delta only changes SLICES length 18 to 19 and appends slice-15. git diff --check and the exact-four-path assertion both exited 0.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 ADVISORY GUARD RED/GREEN PASS. Independent candidate cargo xtask guard with its own target exited 0 in 14.87s wall: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all pass; verdict Pass. Re-pinning only rmcp to =3.8.0 made the guard exit 1 in 15.67s wall with verdict Refused: both cargo_deny and cargo_audit named RUSTSEC-2026-0462, and cargo_deny printed 'Unbounded response body buffering in Streamable HTTP clients'. Exact pin/lock restoration plus git diff --exit-code was clean; the restored guard again exited 0 with verdict Pass in 16.71s wall. The mutant was never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 HOST/LINUX NAME-IDENTITY PASS. Reopened 20261110T005350Z-macos.json: candidate 7ad31eb, tree_clean true, green, 182 passed/0 failed/0 ignored, 182 unique normalized all-ok names, fmt/clippy/tests/guard pass and guard_verdict pass. Reopened 20261110T010817Z-linux-vm.json without new VM contact: same commit, tree_clean true, green_with_declared_limitations, 184 passed/0 failed/0 ignored, 184 unique normalized all-ok names. After stripping ' [ok]' on both sides, comm against pre-bump records 20261108T150008Z-macos.json and 20261108T151334Z-linux-vm.json produced zero differences on either platform. Host-versus-VM comm has exactly one host-only name (holden-cli::tests::readme_quickstart_leads_to_green_check) and three VM-only names (holden-embed::tests::non_tty_caller_cannot_approve, holden-seal::linux::tests::sealed_roundtrip_uses_argon2id_and_xchacha20poly1305, holden-seal::linux::tests::unseal_requires_interactive_tty). The VM's only guard limitations are cargo_deny_absent, cargo_audit_absent and charter_absent, covered by the independently passing host guard; all comparison assertions exited 0.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 PRODUCT REGRESSION PASS. Against the generic private real-repo seed, own-target HOLDEN_E2E_REAL_REPO with timeout -k 10 900 ran the four real_repo tests serially: exit 0, 4 passed/0 failed/0 ignored, 238.33s test time and 256.08s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. A separate own-target focused run of holden-mcp --test workflows exited 0: 12 passed/0 failed/0 ignored in 36.48s test time and 55.04s wall, including e2e_workflows_over_socket, e2e_workflows_over_stdio and all named workflow composition/schema/progress/commit-binding/server-count proofs.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "CHARTER STRICT + FINAL CLEANLINESS PASS. In the clean frozen clone, the canonical pinned release build b7a513b check --strict exited 0 with charter: ok and only the standing accepted-slice cost-row warnings. Every Cargo verification used its own CARGO_TARGET_DIR and a 2700-second outer alarm. After the exact mutant restoration and all test runs, git diff --exit-code and an empty git status --porcelain assertion both exited 0 at the candidate head; no candidate source was modified and no mutant was committed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "Receipt validated with the canonical charter build (commit b7a513b, the project's pinned release build): charter receipt --verify-tree <frozen clone> <receipt> exit 0, verdict accepted.",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": [
    "P1 candidate guard GREEN exit 0 in 14.87s; =3.8.0 mutant RED/Refused exit 1 in 15.67s with cargo-deny and cargo-audit both naming RUSTSEC-2026-0462; exact restore clean and restored guard GREEN exit 0 in 16.71s",
    "P2 supplied host and Linux records reopened and independently normalized/compared: 182/182 and 184/184 pre/post-bump name identities, zero differences; current host/VM asymmetry exactly 1 macOS-only plus 3 Linux-only names, with only the 3 declared VM tool limitations",
    "P3 real_repo independently GREEN 4/4 in 238.33s test time (256.08s wall); workflows independently GREEN 12/12 in 36.48s test time (55.04s wall); corresponding names are [ok] in both supplied gate records",
    "Frozen 363/363 entries matched, exact four-file scope passed, canonical charter check --strict exit 0, and post-proof tree cleanliness passed"
  ]
}
