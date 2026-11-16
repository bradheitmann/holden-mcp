{
  "schema_version": "charter.receipt/1",
  "slice": "slice-18",
  "verifies": "slice-18 independent adversarial QA: frozen identity and bounded scope, declared non-root Cargo crate proofs P1-P8 and P10 with fail-without mutants, real-repository regression, macOS/Boat Linux parity, canonical charter strict check and full guard",
  "candidate_commit": "25e47a1cd527250ea3cffd9da58476792ff950bc",
  "frozen": {
    "manifest_sha256": "d031f604d1943c512e53af523e23b5eb3cd10279a9d4714aad4b158b9e0aec91",
    "head": "25e47a1cd527250ea3cffd9da58476792ff950bc"
  },
  "dispatch": {
    "base_commit": "25e47a1cd527250ea3cffd9da58476792ff950bc",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone was clean at 25e47a1cd527250ea3cffd9da58476792ff950bc with tree 06a0079f95f9cc9fa14ddfe2c9c06c29bb6bfbb7 before QA governance records were copied in. The independently reopened charter.freeze/2 names that head, manifest d031f604d1943c512e53af523e23b5eb3cd10279a9d4714aad4b158b9e0aec91 and 415 entries. Every listed path's Git object id, mode, type and blob SHA-256 was compared; the verification pipeline exited 0 and all 415 matched. DONE SHA-256 and dispatch done_sha256 both equal b62b538417054d4b7181794ff40bc24058fde172b5ccb8257db212f07d18982b; dispatch base is the mandated post-merge tip.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "BOUNDED SCOPE PASS. git diff --check f34fede..25e47a1 exited 0. The complete diff touches only README.md, DONE, the granted holden-cli sources/tests, holden-embed/src/embed.rs, the five granted holden-eval sources, declared-app and declared-tools fixture files, xtask/src/gate.rs, and the two plan rev 13/14 files inherited through the recorded main merges. No other path and no STATE.md change appears. The xtask delta only adds slice-18 and bumps SLICES from 21 to 22.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 DECLARED APP RED/GREEN PASS. On macOS, holden-cli::tests::declared_app_crate_binds_and_completes passed 1/1 (exit 0, 33s wall). Replacing the declaration vector with an empty vector went RED with the exact unsupported_stack refusal (exit 101, 4s wall). Exact revert plus git diff --exit-code was clean and the proof returned GREEN 1/1 (exit 0, 5s wall). The real W2 record passes and contains workspace-tests and cli-0. Source inspection confirms the committed fixture has no root Cargo.toml and a real app/Cargo.toml binary crate; an isolated copy with no ancestor workspace also built offline and locked with exit 0. The Boat Linux gate carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 SEPARATE DECLARED CRATES RED/GREEN PASS. two_declared_crates_bind_separately passed 1/1 (exit 0, 4s wall). Taking only the first declaration went RED with one manifest rather than two (exit 101, 12s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 11s wall). The proof checks distinct declared-tool-a and declared-tool-b entry points, independent workspace-tests and workspace-tests:tools/b/Cargo.toml observations, cli-0 and cli-1, and a pass verdict. The Boat Linux gate carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 EXACT REFUSAL RED/GREEN PASS. undeclared_non_root_refusal_names_manifest passed 1/1 (exit 0, 1s wall), pinning the complete refusal including --manifest <relative-path> and repeatable. Removing the literal flag from production text went RED at the exact-line assertion (exit 101, 6s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 7s wall). The Boat Linux gate carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 DECLARED PATH VALIDATION RED/GREEN PASS. declared_paths_are_validated passed 1/1 (exit 0, 1s wall). Disabling the lexical declaration validation allowed an absolute manifest to embed successfully and went RED at the required refusal (exit 101, 7s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 7s wall). The test independently covers absolute and ../ escape refusals, a Cargo workspace member outside the declared crate, and only the two declared tools manifests while nested/wasm remains absent from the binding. Source inspection confirms nested/wasm is a separate workspace and no production path writes into it. An earlier partial mutant that left the component guard intact stayed green, was not counted as a fail-without control, and was exactly reverted before the effective mutant. The Boat Linux gate carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P5 HOST/BOAT LINUX PARITY PASS. Reopened 20261115T170546Z-macos.json: candidate 25e47a1, green, 201 passed/0 failed/0 ignored, 201 unique all-ok names, guard pass. Host tree_clean false is explained by the dev worktree's untracked dispatch/freeze governance records; all 415 frozen tracked entries matched independently. Reopened host copy 20261115T172955Z-linux-vm.json of VM original 20261115T172448Z-linux.json from the NEW MACHINE Boat gate-19, x86_64 Ubuntu 24.04 (not the earlier arm64 Tart VM): same commit, tree_clean true, green_with_declared_limitations, 200 passed/0 failed/0 ignored, 200 unique all-ok names. Independent comm after stripping [ok] found exactly four macOS-only names (migrate_removes_legacy_key_after_verify, readme_install_covers_holden_mcp, readme_quickstart_leads_to_green_check and written_mcp_json_drives_w1_through_w4) and the three standing Linux-only names (non_tty_caller_cannot_approve, sealed_roundtrip_uses_argon2id_and_xchacha20poly1305 and unseal_requires_interactive_tty). P1-P4 and P6-P8 are [ok] on both. The only VM limitations are cargo_deny_absent, cargo_audit_absent and charter_absent, all covered by the independently passing host guard.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P6 HELP EXIT CODES RED/GREEN PASS. help_exit_codes_are_pinned passed 1/1 (exit 0, 1s wall), covering holden --help, holden help, embed --help and a real usage error. Sending help usage to stderr went RED at the stdout assertion (exit 101, 2s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 1s wall). The Boat Linux gate carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P7 CWD-INDEPENDENT LENS RED/GREEN PASS. lens_default_is_cwd_independent passed 1/1 (exit 0, under 1s wall), invoking embed from the fixture root rather than the host or holden source root. Restoring the source-tree-relative default lens went RED with entity not found (exit 101, 3s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 2s wall). The Boat Linux gate carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "P8 PRE-EXISTING PRIVATE ROOT REFUSAL CLEANUP RED/GREEN PASS. refused_embed_leaves_no_host_state passed 1/1 (exit 0, 1s wall). The test creates both the explicit state and private root before refusal, places sentinels in each, snapshots both trees, and asserts no empty runs directory, no holden-mcp/hosts entry, byte-identical remaining contents and no .holden host state; it then compares fix-and-retry public files with a clean first embed. Moving state directory creation before binding validation went RED precisely at refusal created an empty runs directory in existing state (exit 101, 3s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 3s wall). This reopens the strengthened live-evidence case, not merely the older absent-root case. The Boat Linux gate carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "P10 (DONE proof 9) EMBED CONTRACT DOCUMENTATION RED/GREEN PASS. embed_usage_documents_the_contract passed 1/1 (exit 0, under 1s wall), checking help and README statements for the shared worktree hook, charter planning detection, metadata-only embed, post-commit/W2 build and test, --state default and --manifest. Removing the metadata-only statement went RED at the usage assertion (exit 101, 1s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 1s wall). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "REAL-REPOSITORY REGRESSION PASS. Against the generic private real-repo seed, HOLDEN_E2E_REAL_REPO=<private seed> with the one shared target and timeout -k 10 900 ran all four real_repo tests serially: exit 0, 4 passed, 0 failed, 0 ignored, 218.36s test time and 221s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. Source inspection confirms each test clones the sacred seed afresh with git clone --quiet --no-local into an isolated temporary host; an independent seed git status was empty after the run.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "CHARTER STRICT + GUARD + CLEANLINESS PASS. In the frozen clone, $HOME/.local/bin/charter build 2775000 check --strict exited 0 with charter: ok and only standing accepted-slice cost-row warnings (1s wall). Under set -o pipefail, CARGO_TARGET_DIR=/tmp/holden-s18-target cargo xtask guard exited 0 in 9s and verdict Pass: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all passed. Every foreground proof used the shared target and a 2700-second outer alarm. Every exact mutant revert passed git diff --exit-code; no mutant was committed and no tracked candidate byte remains modified.",
      "classification": "executed"
    },
    {
      "id": "C14",
      "description": "CANONICAL REOPEN + RECEIPT/TREE VALIDATION PASS WITH RECORDED OWNER SEAT LIFT. Canonical $HOME/.local/bin/charter build 2775000 reopen --proof all --run-id 20261115T170546Z-macos --evidence <host gate record> slice-18 exited 0 and recorded proofs 1-9 with evidence SHA-256 e4bab606b55964a1958345a0f5c758be5ec252e41bc6873018008cb9ce95ec64. Canonical receipt --verify-tree with --allow-same-family qa/slice-18/mission-qa-seat-applicability.md exited 0, verdict accepted, naming the matching head and manifest. The applicability file quotes the existing owner decision that from slice-7 QA on MiniMax and DeepSeek are the different-family QA/worker seats; it grants no new bar. The exact unlifted command was also executed and exited 1 solely because build 2775000's family table does not know the mandated minimax-m3.1-flash id. No candidate bytes, model ids or acceptance bars were changed.",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D-QA-TOOL-2",
      "severity": "info",
      "status": "open",
      "description": "Canonical charter build 2775000 omits MiniMax from its built-in model-family table, so the exact unlifted validation command exited 1 and receipts naming the mandated verifier id require the tool's sanctioned --allow-same-family lift. The owner explicitly assigned MiniMax and DeepSeek as different families from slice-7 QA on. The slice-specific owner-decision applicability note makes that existing ruling explicit and grants no new acceptance bar. This is the known validator/tool-table owner item, not a candidate defect."
    }
  ],
  "new_defects": [],
  "proofs_reopened": [
    "P1 host 1/1 in 33s; ignore-declaration mutant RED exit 101 in 4s with UnsupportedStack, exact revert GREEN 1/1 in 5s; Boat Linux gate [ok]",
    "P2 host 1/1 in 4s; first-manifest-only mutant RED exit 101 in 12s, exact revert GREEN 1/1 in 11s; Boat Linux gate [ok]",
    "P3 host 1/1 in 1s; changed-refusal mutant RED exit 101 in 6s, exact revert GREEN 1/1 in 7s; Boat Linux gate [ok]",
    "P4 host 1/1 in 1s; accept-absolute mutant RED exit 101 in 7s, exact revert GREEN 1/1 in 7s; Boat Linux gate [ok]",
    "P5 independent comm compared 201 macOS and 200 Boat Linux unique all-ok names with exactly four host-only and three Linux-only standing names; P1-P4 and P6-P8 pass on both",
    "P6 host 1/1 in 1s; usage-to-stderr mutant RED exit 101 in 2s, exact revert GREEN 1/1 in 1s; Boat Linux gate [ok]",
    "P7 host 1/1 in under 1s; cwd-relative-lens mutant RED exit 101 in 3s with entity not found, exact revert GREEN 1/1 in 2s; Boat Linux gate [ok]",
    "P8 host 1/1 in 1s; partial-state mutant RED exit 101 in 3s at the pre-existing-state empty-runs assertion, exact revert GREEN 1/1 in 3s; Boat Linux gate [ok]",
    "P10 (DONE proof 9) host 1/1 in under 1s; removed-contract-statement mutant RED exit 101 in 1s, exact revert GREEN 1/1 in 1s",
    "Real-repository regression 4/4 in 221s wall; canonical strict check exit 0 and guard exit 0/verdict Pass; canonical reopen exit 0 and lifted verify-tree exit 0/accepted"
  ]
}
