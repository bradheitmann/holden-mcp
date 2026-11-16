{
  "schema_version": "charter.receipt/1",
  "slice": "slice-17",
  "verifies": "slice-17 independent adversarial QA: frozen identity and bounded scope, path-free host-file proofs P1-P7 with fail-without mutants, disposable-keychain custody, real-repository regression, host/Linux evidence, canonical charter strict check and full guard",
  "candidate_commit": "9a6b8b03c6b014a580409dfb397b7a1afd664eaa",
  "frozen": {
    "manifest_sha256": "cef628a8e0723963fc908164fc49b7531aa8cd4f8b3ea3033a87b2e2160e4bf7",
    "head": "9a6b8b03c6b014a580409dfb397b7a1afd664eaa"
  },
  "dispatch": {
    "base_commit": "d3f674fd26175b9252e43446ed161f7330c1677e",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at 9a6b8b03c6b014a580409dfb397b7a1afd664eaa with tree 82b7770afb75c2e21913dad696f133df6106b5be. The independently reopened charter.freeze/2 names that head, manifest cef628a8e0723963fc908164fc49b7531aa8cd4f8b3ea3033a87b2e2160e4bf7 and 387 entries. Every listed path's Git object id, mode, type and blob SHA-256 was compared; the verification pipeline exited 0 and all 387 matched. DONE SHA-256 and dispatch done_sha256 both equal 59652ddf3b03f6367563c4e0d88c8b89ceac000235d8af23d87e41de72334669; dispatch base matches this receipt.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "SCOPE PASS WITH INHERITED-MAIN OBSERVATION. git diff --check c6af813..9a6b8b0 exited 0. That complete diff names the granted README, DONE, holden-cli, holden-embed, holden-mcp workflow/test and xtask paths, plus plans/2026-11-06-integration/dispatch-graph.json. The plan path is not candidate feature work: it entered through the recorded merge of accepted main d3f674f (plan revisions 9-11), and the dispatch-base d3f674f..9a6b8b0 delta touches only the final fixture-scoped AGENT_PRIVATE_ROOT test repair in holden-embed/tests/embed.rs. The xtask delta only adds slice-17 and bumps SLICES from 19 to 20. The amendment-3 workflow delta only validates the relative state alias, resolves the user state root and canonicalizes the alias; no unrelated behavior appears.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 HOST-FILE MACHINE-PATH RED/GREEN PASS. On macOS, fully qualified holden-embed tests::host_files_carry_no_machine_paths passed 1/1 (exit 0, 183.61s wall including the initial build; 113.21s test time). Restoring a root-derived absolute --socket argument in the generated harness entries went RED at the host-file assertion (exit 101, 38.14s wall). Exact revert plus git diff --exit-code was clean and the same test returned GREEN 1/1 (exit 0, 29.22s wall). Source inspection confirms the test computes HOME and USER/LOGNAME from the environment and examines every newly written regular host file outside Git's object store for the fixture root, home, user and absolute filesystem prefixes; the written .mcp.json is in that set. The supplied Linux record independently carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 CROSS-ROOT BYTE-IDENTITY RED/GREEN PASS. mcp_json_byte_identical_across_roots passed 1/1 (exit 0, 9.14s wall). Embedding each fixture's distinct absolute state path into its written .mcp.json made the byte comparison RED (exit 101, 26.59s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 25.16s wall). The supplied Linux record carries the same proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 WRITTEN-CONFIG W1-W4 PASS. On macOS, holden-cli tests::written_mcp_json_drives_w1_through_w4 passed 1/1 (exit 0, 74.36s wall). The proof installs from the README, spawns the real rmcp stdio client using only the exact written .mcp.json command/args, and observes OWNER_APPROVED, CHECK_COMPLETE, NO_FAILURE and OWNER_ACTIONS_CLEAR across W1-W4. This test is one of the four declared macOS-only names, so there is no Linux instance of this name in the VM record; that absence matches the declared platform asymmetry rather than a skipped or failed Linux result.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 MIGRATION/IDEMPOTENCE RED/GREEN PASS. reembed_migrates_old_mcp_json_and_keeps_approvals passed 1/1 (exit 0, 7.32s wall), covering legacy absolute-socket migration, a second byte-identical embed, unchanged approval records and renewed admission. Restoring --socket with an absolute path in generated entries made the migrated-args assertion RED (exit 101, 25.43s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 22.68s wall). The supplied Linux record carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P5 README INSTALL RED/GREEN PASS. On macOS, readme_install_covers_holden_mcp passed 1/1 (exit 0, 90.47s wall), installing both binaries in a fresh clone and invoking holden-mcp through the installed PATH. Removing only the README's holden-mcp cargo-install line went RED because install/bin/holden-mcp was absent (exit 101, 40.35s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 45.10s wall). This is a declared macOS-only test. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P6 VERIFY-THEN-REMOVE + KEYCHAIN CUSTODY RED/GREEN PASS. On macOS, migrate_removes_legacy_key_after_verify passed 1/1 (exit 0, 5.86s wall): a different disposable Keychain key causes refusal while the disposable legacy owner.key remains, then matching migration removes it and preserves owner.pub. Deleting the legacy file before the migrated-signature comparison went RED (exit 101, 9.59s wall); exact revert was clean and the proof returned GREEN 1/1 (exit 0, 9.74s wall). Source audit of migration, quickstart, embed, real-repo, server-run and workflow tests found only named throwaway keychains beneath fixture owner stores, with throwaway passwords and Drop/temp-root cleanup; no test invokes default_keychain or writes the real login keychain. The service string occurs only in the production Keychain implementation. The mutant also used only that disposable keychain. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P7 DROP-GUARD RED/GREEN + LEAK CHECK PASS. fixture_temp_dirs_are_drop_guard_cleaned passed 1/1 (exit 0, 1.77s wall), first demonstrating that an unguarded forced panic leaks and then asserting that the guarded fixture does not. Removing impl Drop for Fx made the guarded-leak assertion RED (exit 101, 6.85s wall). Exact revert was clean and the proof returned GREEN 1/1 (exit 0, 7.81s wall). After all proof, real-repository and guard runs, an independent find over both /tmp and the process temp root found zero holden-501-* directories. The supplied Linux record carries this proof [ok]. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "REAL-REPOSITORY REGRESSION PASS. Against the generic private real-repo seed, own-target HOLDEN_E2E_REAL_REPO=<private seed> with timeout -k 10 900 ran all four real_repo tests serially: exit 0, 4 passed, 0 failed, 0 ignored, 474.74s test time and 482.22s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. Source inspection confirms every test clones the seed afresh with git clone --no-local into an isolated temporary host; no test writes to the sacred seed.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "HOST/LINUX EVIDENCE + EXACT PLATFORM ASYMMETRY PASS. Reopened 20261115T121034Z-macos.json: candidate 9a6b8b0, green, 189 passed/0 failed/0 ignored, 189 unique all-ok names, guard pass. Host tree_clean false is explained by the dev worktree's three untracked dispatch/freeze governance records; all 387 frozen tracked entries matched independently. Reopened 20261115T123758Z-linux-vm.json without new VM contact: same commit, tree_clean true, green_with_declared_limitations, 188 passed/0 failed/0 ignored, 188 unique all-ok names. Independent comm after stripping [ok] found exactly four host-only and three Linux-only names. The host-only migration test's actual integration-test id is holden-cli::migrate_removes_legacy_key_after_verify (without tests::); the other host-only ids are tests::readme_install_covers_holden_mcp, tests::readme_quickstart_leads_to_green_check and tests::written_mcp_json_drives_w1_through_w4. Linux-only ids are non_tty_caller_cannot_approve and the two holden-seal Linux keystore tests. The only VM limitations are cargo_deny_absent, cargo_audit_absent and charter_absent, all covered by the independently passing host guard. P1, P2, P4 and P7 are [ok] on Linux; P3/P5/P6 are declared host-only. The retained round-one record 20261115T120758Z-linux-vm.json is red 187/188 with host_implementer_cannot_reveal [FAILED], preserving the pre-fix fail state; the final record is green at the fixed tip.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "CHARTER STRICT + GUARD + CLEANLINESS PASS. In the clean frozen clone, $HOME/.local/bin/charter build 2775000 check --strict exited 0 with charter: ok and only standing accepted-slice cost-row warnings (1.15s wall). Under set -o pipefail, CARGO_TARGET_DIR=/tmp/holden-s17-target cargo xtask guard exited 0 in 20.07s and verdict Pass: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all passed. Every foreground proof used the shared target and a 2700-second outer alarm. Every exact mutant revert passed git diff --exit-code; no mutant was committed and no tracked candidate byte remains modified.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "CANONICAL RECEIPT/TREE VALIDATION PASS WITH RECORDED OWNER SEAT LIFT; UNLIFTED TOOL GAP PRESERVED. After copying the supplied dispatch/freeze governance records into the QA clone, canonical $HOME/.local/bin/charter build 2775000 reopen --proof all --run-id 20261115T121034Z-macos exited 0 and recorded proofs 1-7 with evidence SHA-256 bcb1bde88844743eeec056b48b234fa5cee18351cb65dbf5f3c36f35d18ab4d2. Canonical receipt --verify-tree with --allow-same-family qa/slice-17/mission-qa-seat-applicability.md then exited 0, verdict accepted, naming the matching head and manifest. The applicability file quotes the existing owner decision that from slice-7 QA on MiniMax and DeepSeek are the different-family QA/worker seats; it grants no new bar. The exact unlifted receipt --verify-tree command was also executed and exited 1 solely because build 2775000's family table does not know the mandated minimax-m3.1-flash id; the same build rejects the accepted slice-14 receipt for that same model-table reason. No candidate bytes, model ids or acceptance bars were changed.",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D-QA-BRIEF-1",
      "severity": "info",
      "status": "resolved",
      "description": "Two briefing notation mismatches were reopened without changing the candidate or bar: the complete pre-merge-base diff also shows the dispatch graph inherited from accepted main (the dispatch-base delta has no such file), and the actual macOS integration-test id for P6 omits the briefing's extra tests:: namespace component. The inherited-main provenance and independently executed P6 name are recorded in C02/C11; neither is a product defect."
    },
    {
      "id": "D-QA-TOOL-2",
      "severity": "info",
      "status": "open",
      "description": "Canonical charter build 2775000 omits MiniMax from its built-in model-family table, so the exact unlifted validation command exits 1 even though the owner has explicitly assigned MiniMax and DeepSeek as different families from slice-7 QA on. The canonical owner-decision lift validates this receipt and frozen tree with exit 0/accepted. This is a validator/tool-table gap, not a candidate defect; the unlifted exit-1 observation is not hidden."
    }
  ],
  "new_defects": [],
  "proofs_reopened": [
    "P1 host 1/1; absolute-socket mutant RED exit 101 in 38.14s, exact revert GREEN 1/1 in 29.22s; Linux gate [ok]",
    "P2 host 1/1; per-root path mutant RED exit 101 in 26.59s, exact revert GREEN 1/1 in 25.16s; Linux gate [ok]",
    "P3 host written-config rmcp W1-W4 1/1 in 74.36s; declared macOS-only in the host/Linux comm",
    "P4 host 1/1; restored-legacy-socket mutant RED exit 101 in 25.43s, exact revert GREEN 1/1 in 22.68s; Linux gate [ok]",
    "P5 host 1/1; remove-holden-mcp-install mutant RED exit 101 in 40.35s, exact revert GREEN 1/1 in 45.10s; declared macOS-only",
    "P6 host 1/1; remove-before-signature-verify mutant RED exit 101 in 9.59s, exact revert GREEN 1/1 in 9.74s; disposable-keychain audit passed",
    "P7 host 1/1; missing-Drop mutant RED exit 101 in 6.85s, exact revert GREEN 1/1 in 7.81s; Linux gate [ok] and zero holden-501-* leaks",
    "Real-repository regression 4/4 in 482.22s wall; host gate 189/0/0 and Linux gate 188/0/0 reopened; canonical strict check exit 0 and guard exit 0/verdict Pass"
  ]
}
