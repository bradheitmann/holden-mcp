{
  "schema_version": "charter.receipt/1",
  "slice": "slice-12b",
  "verifies": "slice-12b QA: frozen Linux owner parity, non-TTY fail-closed unsealing, pre-migration compatibility, owner migration documentation, real-repository workflows, host and Linux gate records",
  "candidate_commit": "8ae1ef1deaaf1af4f779dfd0f8e2752f54c4f948",
  "frozen": {
    "manifest_sha256": "86c0a4d741b1a809063e50f62315215f5fed47302d19a19d38747f82af295fda",
    "head": "8ae1ef1deaaf1af4f779dfd0f8e2752f54c4f948"
  },
  "dispatch": {
    "base_commit": "357339794a0fcac3c2e98bb27ccfcd631ec52d72",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at 8ae1ef1deaaf1af4f779dfd0f8e2752f54c4f948 with tree a58bbd1d4b65a2a196a5ebaaad1fc5f45cf76298. The independently reopened charter.freeze/2 names that head/tree, manifest 86c0a4d741b1a809063e50f62315215f5fed47302d19a19d38747f82af295fda and 338 entries. Every entry's Git object id, mode, type and SHA-256 content digest matched the clone (exit 0, 338/338). DONE sha256 and the current dispatch done_sha256 are both 0358d0075105f2a169187d70f4331ceb4b94bfd1aead4afd494b70d1f119e309; dispatch base is 357339794a0fcac3c2e98bb27ccfcd631ec52d72.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "SCOPE AUDIT PASS. The independently computed 24a42fb..8ae1ef1 delta is exactly the 15 granted paths: Cargo.toml/Cargo.lock, README.md, the slice-12b DONE list, readme.rs, approve.rs, embed.rs, real_repo.rs, workflows.rs, holden-seal Cargo.toml plus keystore.rs/lib.rs/linux.rs/owner.rs, and xtask/src/gate.rs. The xtask delta only adds slice-12b to SLICES and changes its array length from 15 to 16. git diff --check exited 0; no out-of-scope path exists.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 IDENTICAL-NAME OWNER PARITY PASS. In the clean frozen macOS clone, the focused embed run exited 0: owner_commands_require_presence_and_bars_stay_refused and owner_key_resolves_through_keystore_not_file both passed, 2 passed/0 failed/0 ignored in 29.12s test time (50.11s wall). They used real disposable Keychain state and real PTY owner commands. Reopened VM record 20261108T103802Z-linux-vm.json contains both identical fully qualified names with [ok]; its Linux approval path uses a disposable passphrase-sealed store through a real PTY.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 NON-TTY REFUSAL + FAIL-WITHOUT PASS. Reopened on-VM record 20261108T104111Z-linux-vm-p2-mutant.json at 8ae1ef1: the documented unseal-bypass mutant made holden-seal linux::tests::unseal_requires_interactive_tty RED exit 101 (0.51s) and holden-embed tests::non_tty_caller_cannot_approve RED exit 101 (23.93s), each 0 passed/1 failed. Exact revert was clean; both returned GREEN exit 0, 1/1, in 0.83s and 24.90s. Final VM gate record at the same commit carries both names [ok]. Classification is from these executed VM records; no second VM mutant run was needed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "HOST/VM EVIDENCE + EXACT ASYMMETRY PASS. Reopened 20261108T102236Z-macos.json: 8ae1ef1, green, 174 unique names all [ok], 174 passed/0 failed/0 ignored, guard pass. Reopened 20261108T103802Z-linux-vm.json: the same commit, tree_clean true, green_with_declared_limitations, 176 unique names all [ok], 176 passed/0 failed/0 ignored, guard pass_with_declared_limitations. Independent comm produced exactly one host-only name, holden-cli::tests::readme_quickstart_leads_to_green_check [ok], and exactly three VM-only names: non_tty_caller_cannot_approve, unseal_requires_interactive_tty and sealed_roundtrip_uses_argon2id_and_xchacha20poly1305, all [ok]. These are declared typed cfg platform asymmetries, not failures. Host tree_clean false is explained solely by the three untracked slice-12b dispatch/freeze governance files; tracked diff is empty.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P3 PRE-MIGRATION COMPATIBILITY RED/GREEN PASS. The exact macOS test owner::tests::pre_migration_approvals_still_verify initially passed 1/1 in 0.01s. Rejecting only the pre-migration message in OwnerPublicKey::verify made it RED exit 101 with BadSignature, 0/1 in 0.02s. Exact revert gave an empty Git diff and the same test went GREEN exit 0, 1/1 in 0.02s. VM record 20261108T103802Z-linux-vm.json independently contains holden-seal::owner::tests::pre_migration_approvals_still_verify [ok]. Mutant never committed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P4 OWNER MIGRATION DOCUMENTATION RED/GREEN PASS. The exact macOS readme_names_the_owner_migration_command initially passed 1/1. Removing the complete Owner-only key migration section made it RED exit 101, 0/1, on the missing section. Exact revert gave an empty Git diff and GREEN exit 0, 1/1. Source audit found the only executable migration test call is the headless refusal against the test fixture's disposable owner store; no test invokes migration against a real owner key, and all PTY approvals use disposable stores with throwaway passphrases. Mutant never committed.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "REAL-REPOSITORY REGRESSION PASS. In the clean frozen clone, the four real_repo tests ran serially against the private seed with an isolated target and timeout -k 10 900: exit 0, 4 passed/0 failed/0 ignored in 233.94s test time and 257.13s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. Source inspection confirms every test clones the seed afresh with git clone --no-local; the seed was never written.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "LINUX CUSTODY CONSTRUCTION AUDIT PASS. LinuxPassphraseKeyStore stores owner.sealed.json, derives a 32-byte key with Argon2id and encrypts with XChaCha20-Poly1305 using fresh OS-random salt/nonce and versioned authenticated data. get resolves the interactive passphrase before reading sealed bytes; rpassword reads the controlling terminal with echo disabled. There is no stdin, environment-variable, passphrase-file or Secret Service fallback. Sealed files require regular-file identity, owner/mode checks and 0600 permissions; wrong passphrases fail authenticated decryption. macOS Keychain behavior remains independently green under P1 and the real-repository suite.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "DECLARED LINUX LIMITATIONS + FAIL-STATE EVIDENCE PASS. Final VM guard has exactly cargo_deny_absent, cargo_audit_absent and charter_absent as limitations; every other VM guard check passes and the independently executed macOS guard covers all three. Reopened round-1 record 20261108T100154Z-linux-vm.json is deliberately RED at 3573397: 171 passed/5 failed/0 ignored, with exactly the five workflow tests failed before their PTY migration. Final record 20261108T103802Z-linux-vm.json has those same five names [ok] within 176/176. This preserves the refusal-regression evidence without treating the repaired round as a candidate failure.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "CHARTER + GUARD PASS. The canonical pinned charter build's check --strict at the frozen tip exited 0 with charter: ok and only the standing accepted-slice cost-row warnings. Standalone cargo xtask guard with its own CARGO_TARGET_DIR exited 0 under set -o pipefail: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all pass; verdict Pass. QA mutants were exactly reverted, never committed, and the candidate tracked tree was clean before receipt creation.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "Receipt validated with the canonical charter build (commit b7a513b, the project's pinned release build): charter receipt --verify-tree <clone> <receipt> exit 0, verdict accepted",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": [
    "P1 macOS executed independently 2/2 in 29.12s; both identical Linux names reopened [ok] in 176/176 VM gate",
    "P2 Linux unseal-bypass mutant RED exit 101/101 (0.51s/23.93s), exact clean revert, GREEN exit 0/0 1/1 each (0.83s/24.90s); final VM [ok] records reopened",
    "P3 macOS executed 1/1; rejection mutant RED exit 101 then exact revert GREEN exit 0 1/1; Linux [ok] reopened",
    "P4 macOS README contract executed 1/1; section-removal mutant RED exit 101 then exact revert GREEN exit 0 1/1; disposable-store migration audit completed",
    "Four real-repository tests executed serially 4/4 in 233.94s; canonical charter check --strict exit 0 and standalone guard exit 0 verdict Pass"
  ]
}
