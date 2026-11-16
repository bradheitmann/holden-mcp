{
  "schema_version": "charter.receipt/1",
  "slice": "slice-18a",
  "verifies": "slice-18a independent adversarial QA: frozen identity and bounded scope, refused-embed detection-report proofs P1-P5 with fail-without mutants, host/Boat Linux parity, real-repository regression, path-scan integrity, canonical charter strict check and full guard",
  "candidate_commit": "ea2615b1d96a2940b3b6d16ef73c33fcc7a85950",
  "frozen": {
    "manifest_sha256": "58426cc3e96952f5440b562342a27fcd068351841df8d34d0d4fe81128c45262",
    "head": "ea2615b1d96a2940b3b6d16ef73c33fcc7a85950"
  },
  "dispatch": {
    "base_commit": "c57180b51dffa6644facbcdc5d66f7fbaee02f67",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at ea2615b1d96a2940b3b6d16ef73c33fcc7a85950 with tree 4391b933d5782024c2b4bc6683361cd1fdfe93f7. The independently reopened charter.freeze/2 names that head/tree, manifest 58426cc3e96952f5440b562342a27fcd068351841df8d34d0d4fe81128c45262 and 394 entries. Every listed path's Git object id, mode, type and blob SHA-256 was compared; the verification pipeline exited 0 and all 394 matched. DONE SHA-256 and the supplied dispatch done_sha256 both equal 11a97920d3c1a14aedcd525e8d51de75886e7ccfd21e3aecb9d4acf735dafbb7. This receipt uses the mandated post-merge dispatch base c57180b51dffa6644facbcdc5d66f7fbaee02f67.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "SCOPE PASS WITH ONE EXPLICITLY FLAGGED BRIEFING-LIST DELTA. git diff --check f32b142..ea2615b exited 0. The complete diff touches DONE, holden-embed/src/embed.rs, holden-cli/src/embed_cmd.rs, the new holden-cli/tests/embed_report.rs, holden-embed/tests/embed.rs, xtask/src/gate.rs and plans/2026-11-06-integration/dispatch-graph.json; charter/STATE.md is untouched. The plan change is the recorded merge of accepted main 0a76b80 (plan rev 12). The xtask delta only adds slice-18a and bumps SLICES from 20 to 21. The one path outside the briefing's exhaustive enumeration is holden-embed/tests/embed.rs; it is precisely the briefed whole-word account-name test repair, within the DONE resource grant for tests, and the c57180b..ea2615b delta touches only that file. No production behavior outside the two granted source files appears. D-QA-BRIEF-1 records the enumeration omission without hiding it.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 REFUSAL-REPORT RED/GREEN PASS. On macOS, holden-cli::tests::refused_embed_still_prints_detection_report passed 1/1 (exit 0, 20.71s wall including the initial build; 12.79s test time). Dropping the report from the refusal path went RED at the missing planning-process assertion (exit 101, 2.19s wall). Exact revert plus git diff --exit-code was clean and the proof returned GREEN 1/1 (exit 0, 1.53s wall). A separate real CLI observation exited 1 and printed planning process markdown, README.md and docs/PRD.md, candidate count 2 and both complete candidate texts on lines 1-6, before the unsupported_stack refusal on line 7. The supplied Boat Linux record carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 SHAPE-PARITY RED/GREEN PASS. refusal_report_matches_success_shape passed 1/1 (exit 0, 1.19s wall). Dropping the candidate-count field from only the refusal printer went RED with refusal shape [planning process, source document, candidate promise] versus success shape [planning process, source document, candidate promises, candidate promise] (exit 101, 1.81s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 2.04s wall). The supplied Boat Linux record carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 NO-PROMISE-REFUSAL RED/GREEN PASS. no_promise_refusal_prints_no_report passed 1/1 (exit 0, 1.07s wall), with exact stderr limited to the unchanged no-owner-promise refusal. Setting an empty detection report on the no-promise path went RED: output gained planning-process, source-document and zero-count report lines (exit 101, 2.65s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 2.18s wall). The supplied Boat Linux record carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 NO-MACHINE-VALUES RED/GREEN PASS. detection_report_carries_no_machine_values passed 1/1 (exit 0, 1.13s wall), covering both success and refusal. Adding the state directory as a report source document went RED at the machine-path assertion (exit 101, 2.77s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 3.20s wall). The test also rejects socket names, the sealed owner-key marker and absolute document paths. The supplied Boat Linux record carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P5 HOST/BOAT LINUX PARITY + FAIL-STATE REOPEN PASS. Reopened 20261115T151334Z-macos.json: candidate ea2615b, green, 193 passed/0 failed/0 ignored, guard pass. Host tree_clean false is explained by the dev worktree's three untracked dispatch/freeze governance records; all 394 frozen tracked entries matched independently. Reopened 20261115T153529Z-linux-vm.json from the NEW MACHINE Boat gate-19, x86_64 Ubuntu 24.04 (not the earlier arm64 Tart VM): same commit, tree_clean true, green_with_declared_limitations, 192 passed/0 failed/0 ignored. Independent comm after stripping [ok] found exactly four macOS-only names (migrate_removes_legacy_key_after_verify, readme_install_covers_holden_mcp, readme_quickstart_leads_to_green_check and written_mcp_json_drives_w1_through_w4) and the three standing Linux-only names (non_tty_caller_cannot_approve, sealed_roundtrip_uses_argon2id_and_xchacha20poly1305 and unseal_requires_interactive_tty). P1-P4 are [ok] on both. The only VM limitations are cargo_deny_absent, cargo_audit_absent and charter_absent, all covered by the independently passing host guard. Retained RED records 20261115T145338Z-linux-vm.json (187 passed/5 failed: the unstaged-seed real-repository could_not_run set plus the P1 false positive) and 20261115T150650Z-linux-vm.json (191 passed/1 failed: only the P1 false positive) preserve the repair trail at c57180b; the final Boat record is green at ea2615b.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "REAL-REPOSITORY REGRESSION PASS. Against the generic private real-repo seed, own-target HOLDEN_E2E_REAL_REPO=<private seed> with timeout -k 10 900 ran all four real_repo tests serially: exit 0, 4 passed, 0 failed, 0 ignored, 211.60s test time and 213.44s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. Source inspection confirms every test clones the sacred seed afresh with git clone --quiet --no-local into an isolated temporary host; no test writes to the seed.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P1-SCAN INTEGRITY RED/GREEN PASS. The whole-word account-name repair did not weaken slice-17's machine-path scan. holden-embed::tests::host_files_carry_no_machine_paths passed 1/1 (exit 0, 3.49s wall). Restoring an absolute --socket value in generated harness entries went RED at absolute path in .codex/config.toml (exit 101, 2.82s wall), independently of account-name matching. Exact revert was clean and the scan returned GREEN 1/1 (exit 0, 2.43s wall). This reopens the DONE-declared slice-17 absolute-socket fail-without mutant on the fixed scan. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "CHARTER STRICT + GUARD + CLEANLINESS PASS. In the clean frozen clone, $HOME/.local/bin/charter build 2775000 check --strict exited 0 with charter: ok and only standing accepted-slice cost-row warnings (0.56s wall). Under set -o pipefail, CARGO_TARGET_DIR=/tmp/holden-s18a-target cargo xtask guard exited 0 in 6.15s and verdict Pass: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all passed. Every foreground proof used the one shared target and a 2700-second outer alarm. Every exact mutant revert passed git diff --exit-code; no mutant was committed and no tracked candidate byte remains modified.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "CANONICAL REOPEN + RECEIPT/TREE VALIDATION PASS WITH RECORDED OWNER SEAT LIFT. Canonical $HOME/.local/bin/charter build 2775000 reopen --proof all --run-id 20261115T151334Z-macos exited 0 and recorded proofs 1-5 with evidence SHA-256 7bedc5d7ed2fc82716d38121ec4a800d8092cea9459f38d8689d527c4207df17. Canonical receipt --verify-tree with --allow-same-family qa/slice-18a/mission-qa-seat-applicability.md exited 0, verdict accepted, naming the matching head and manifest. The applicability file quotes the existing owner decision that from slice-7 QA on MiniMax and DeepSeek are the different-family QA/worker seats; it grants no new bar. The exact unlifted receipt command was also executed and exited 1 solely because build 2775000's family table does not know the mandated minimax-m3.1-flash id. No candidate bytes, model ids or acceptance bars were changed.",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D-QA-BRIEF-1",
      "severity": "info",
      "status": "resolved",
      "description": "The briefing's exhaustive scope-path enumeration omitted crates/holden-embed/tests/embed.rs even though the same briefing explicitly describes its whole-word P1-scan test repair and the DONE resource scope grants tests. The full branch audit flags that path in C02: its only delta is the described test fix, the post-merge delta contains nothing else, and no production scope expands."
    },
    {
      "id": "D-QA-TOOL-2",
      "severity": "info",
      "status": "open",
      "description": "Canonical charter build 2775000 omits MiniMax from its built-in model-family table, so the exact unlifted validation command exits 1 even though the owner explicitly assigned MiniMax and DeepSeek as different families from slice-7 QA on. The canonical owner-decision lift validates this receipt and frozen tree with exit 0/accepted. This is the known validator/tool-table owner item, not a candidate defect; the unlifted exit-1 observation is not hidden."
    }
  ],
  "new_defects": [],
  "proofs_reopened": [
    "P1 host 1/1 in 20.71s; drop-report mutant RED exit 101 in 2.19s, exact revert GREEN 1/1 in 1.53s; direct CLI exit 1 with all six report lines before refusal; Boat Linux gate [ok]",
    "P2 host 1/1 in 1.19s; drop-count-field mutant RED exit 101 in 1.81s, exact revert GREEN 1/1 in 2.04s; Boat Linux gate [ok]",
    "P3 host 1/1 in 1.07s; print-empty-report mutant RED exit 101 in 2.65s, exact revert GREEN 1/1 in 2.18s; Boat Linux gate [ok]",
    "P4 host 1/1 in 1.13s; include-state-path mutant RED exit 101 in 2.77s, exact revert GREEN 1/1 in 3.20s; Boat Linux gate [ok]",
    "P5 independent comm compared 193 macOS and 192 Boat Linux unique all-ok names with exactly four host-only and three Linux-only standing names; P1-P4 pass on both",
    "P1-scan host 1/1 in 3.49s; absolute-socket mutant RED exit 101 in 2.82s, exact revert GREEN 1/1 in 2.43s",
    "Real-repository regression 4/4 in 213.44s wall; canonical strict check exit 0 and guard exit 0/verdict Pass; canonical reopen exit 0 and lifted verify-tree exit 0/accepted"
  ]
}
