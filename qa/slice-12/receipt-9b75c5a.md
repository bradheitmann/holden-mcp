{
  "schema_version": "charter.receipt/1",
  "slice": "slice-12",
  "verifies": "slice-12 QA: frozen identity, obligation mutants, fresh-clone quickstart, owner-only hardening, real-repository workflows, host and Linux gate records",
  "candidate_commit": "9b75c5a365b94ed3bc6b4c5592dde6d38a676854",
  "frozen": {
    "manifest_sha256": "903a2490b5b6451cd05aae224a454a79dc0bb893210f2f27d9b6530216796c8a",
    "head": "9b75c5a365b94ed3bc6b4c5592dde6d38a676854"
  },
  "dispatch": {
    "base_commit": "134bfa85954aebf14dfe3a2b4e979f212dcb790f",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at 9b75c5a365b94ed3bc6b4c5592dde6d38a676854 with tree 278597177b7df13ed77f9a2a4c0a1dce8a8130a8. charter.freeze/2 independently reopened from the dev worktree names that head/tree, manifest 903a2490b5b6451cd05aae224a454a79dc0bb893210f2f27d9b6530216796c8a and 331 entries; the object id, mode, type and SHA-256 content digest of every entry match the clone (exit 0). DONE sha256 and dispatch done_sha256 are both e1181b461e572c337380a95b67964c12ab01dbecaa6efec1f5788e3bb43f82d9.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "P1a CERT SENTINEL RED/GREEN. Removing !cargo_cert_sentinel from watched_build_env made holden-eval tests::cert_sentinel_exempts_cargo_and_catches_others fail its empty-watch assertion (exit 101, 15.96s). Exact revert produced a clean tree and the fully qualified test passed 1/1 (exit 0, 17.66s). The test also retains changed-value and additional-certificate leak assertions. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1b VM PRIVATE ROOT RED/GREEN. Dropping private_root_export from gate_script made xtask tests::vm_gate_forwards_private_root fail its exact AGENT_PRIVATE_ROOT forwarding assertion (exit 101, 17.31s). Exact revert was clean and passed 1/1 (exit 0, 16.71s). The live Linux run independently reopened with guard private_identifiers pass and the private-root-dependent gate steps green; its result is green_with_declared_limitations. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P1c BOUNDED REAP RED/GREEN. Replacing timeout(REAP_DEADLINE, child.wait()) with child.wait() made holden-cli tests::relay_reap_wait_is_bounded fail after the real child's seven-second exit instead of producing Deadline(server reap) (exit 101, 24.84s wall). Exact revert was clean and passed 1/1 at the five-second deadline (exit 0, 22.81s wall). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P2 QUICKSTART RED/GREEN. Removing only the README setup block's holden init made readme_quickstart_leads_to_green_check fail when the documented denylist write found no initialized private directory (exit 101, 74.37s). Exact revert was clean; in its own fresh inner clone the same test executed the README installs, init, fixture embed, disposable-Keychain approval through a genuine PTY, product commit and real holden_check, returning CHECK_COMPLETE with one pass (exit 0, 1/1, 76.08s). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 KEYCHAIN CUSTODY RED/GREEN. Replacing only the macOS owner backend with FileKeyStore made holden-embed tests::owner_key_resolves_through_keystore_not_file sign with the decoy owner.key and fail the owner public-key inequality assertion (exit 101, 46.62s). Exact revert was clean and passed 1/1 (exit 0, 42.46s): real PTY approvals resolved the disposable named Keychain key, every approval re-admitted against owner.pub, and reopening retained the same public key. Never committed; no real owner key or login keychain was accessed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "CLI OWNER PRESENCE + BARS RED/GREEN. Removing require_owner_terminal from owner_key allowed a headless real holden approval to succeed, so owner_commands_require_presence_and_bars_stay_refused failed with exit Some(0) rather than Some(1) (exit 101, 40.38s). Exact revert was clean and passed 1/1 (exit 0, 42.55s): headless approve and migrate-owner-key both refused owner absence, no signing artifact published, and holden bars remained an unknown command. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "MCP PENDING-OWNER TOOLS RED/GREEN. Disabling owner_required let W2 proceed beyond pending approval, and implementer_owner_step_returns_owner_action_required failed when holden_check returned workflow_unavailable instead of OWNER_ACTION_REQUIRED (exit 101, 20.71s). Exact revert was clean and passed 1/1 (exit 0, 20.52s), covering W2 check, W3 explain, W4 status, absent authority_bars while pending, and refusal of the nonexistent implementer approve tool. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "MCP PROMPTS RED/GREEN. Adding the executable text holden approve to W2-CHECK made prompts_list_all_four_workflows_with_arguments fail W2-CHECK enables holden approve (exit 101, 20.69s). Exact revert was clean and passed 1/1 (exit 0, 20.26s), exercising all four real prompts and rejecting approve, migrate-owner-key, call approve and change-bars-to instruction text. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "SERVER INSTRUCTIONS RED/GREEN. Adding holden approve to INSTRUCTIONS made instructions_name_workflows fail instructions enable owner execution: holden approve (exit 101, 20.48s). Exact revert was clean and passed 1/1 (exit 0, 20.33s), retaining workflow order, the owner-stop contract and all four owner-command/bar-change prohibitions. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "SLASH MAPPING RED/GREEN. Mapping /holden approve to a low-level approval operation made holden_command_maps_to_workflows_only fail with Some(approve) rather than None (exit 101, 20.96s). Exact revert was clean and passed 1/1 (exit 0, 18.50s). The test refuses approve, migrate-owner-key, bars, set-bars, change-bars, owner and other low-level operations and requires owner commands to be printed, never executed. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "REAL-REPOSITORY E2E PASS. In the clean frozen clone, the four real_repo tests ran serially under timeout -k 10 900 with the private seed and an isolated target: exit 0, 4 passed, 0 failed, 0 ignored, 219.78s test time and 239.41s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. Source inspection and execution confirm each run clones the seed with --no-local; the seed was never written.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "HOST/VM EVIDENCE + EXACT ASYMMETRY PASS. Reopened 20261108T083332Z-macos.json: commit 9b75c5a, result green, tree_clean true, 172 unique names all [ok], 0 failed and 0 ignored; guard_verdict pass. Reopened 20261108T084623Z-linux-vm.json: the same commit, tree_clean true, result green_with_declared_limitations, 169 unique names all [ok], 0 failed and 0 ignored; guard_verdict pass_with_declared_limitations. Sorted comm shows exactly three host-only names and zero VM-only names: holden-cli::tests::readme_quickstart_leads_to_green_check, holden-embed::tests::owner_commands_require_presence_and_bars_stay_refused, and holden-embed::tests::owner_key_resolves_through_keystore_not_file, all [ok]. Every other name matches.",
      "classification": "executed"
    },
    {
      "id": "C14",
      "description": "P1d STANDING ACCEPTED LIMITATION FORM. The live Linux guard record explicitly carries only cargo_deny_absent, cargo_audit_absent and charter_absent, each typed limitation, while its other guard checks pass. The independently executed macOS guard runs cargo-deny, cargo-audit and charter safety green. This is the same accepted disposition as slice-9 receipt C13, slice-10 receipt C13 and slice-11 receipt C14: VM-only declared limitations covered by the passing host guard. No new owner-decision waiver is required and no VM state was changed.",
      "classification": "executed"
    },
    {
      "id": "C15",
      "description": "P3 CHARTER + GUARD PASS. The canonical pinned charter build's check --strict at the isolated tip exited 0 with charter: ok and only the standing accepted-slice cost-row warnings. Standalone cargo xtask guard with its own CARGO_TARGET_DIR exited 0: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all pass; verdict Pass. Both commands ran under set -o pipefail.",
      "classification": "executed"
    },
    {
      "id": "C16",
      "description": "SCOPE + CLEANUP PASS. The independently computed 552c8d6..9b75c5a delta is exactly the ten supplied scope files and git diff --check exits 0. Every mutant was edited only in the isolated QA clone, exactly reverted, and followed by git diff --exit-code before its green run. The restored candidate tree is clean; no mutant was committed and the dev worktree was not modified.",
      "classification": "executed"
    },
    {
      "id": "C17",
      "description": "Receipt validated with the canonical charter build (commit b7a513b, the project's pinned release build): charter receipt --verify-tree <clone> <receipt> exit 0, verdict accepted",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": [
    "1",
    "2",
    "3",
    "4",
    "P1 P1a cert sentinel executed independently; exemption-removal mutant RED exit 101 then exact revert GREEN 1/1",
    "P1 P1b VM forwarding executed independently; forwarding-removal mutant RED exit 101 then exact revert GREEN 1/1; live Linux private-root-dependent record reopened",
    "P1 P1c bounded reap executed independently; deadline-removal mutant RED exit 101 then exact revert GREEN 1/1",
    "P1 P1d VM-only guard limitations reopened and matched to the accepted slice-9/10/11 host-covered disposition; no waiver needed",
    "P2 fresh-clone README quickstart executed independently; omit-init mutant RED exit 101 then exact revert GREEN 1/1 with real PTY owner approval and CHECK_COMPLETE/pass",
    "P3 canonical charter check --strict exit 0 and standalone guard exit 0 with verdict Pass",
    "P4 macOS Keychain custody executed independently with decoy legacy key; file-backend mutant RED exit 101 then exact revert GREEN 1/1; owner-only hardening and four real-repository tests also reopened"
  ]
}
