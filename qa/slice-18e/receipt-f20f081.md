{
  "schema_version": "charter.receipt/1",
  "slice": "slice-18e",
  "verifies": "slice-18e independent adversarial QA: DONE and frozen identity, bounded scope, actionable command-output proofs P1-P7 with compiling fail-without mutants, slice-18b compatibility, real-repository regression, independent Linux VM gate and exact platform parity, canonical safety and receipt validation",
  "candidate_commit": "f20f08132be34a25ca76490e3140e1dacbbb95c1",
  "frozen": {
    "manifest_sha256": "4164495a4fab8ebdcd374b3dff6039e016ffc85ea681f38564e3fa26744da2c5",
    "head": "f20f08132be34a25ca76490e3140e1dacbbb95c1"
  },
  "dispatch": {
    "base_commit": "f20f08132be34a25ca76490e3140e1dacbbb95c1",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "DONE AND FROZEN IDENTITY PASS. A fresh clone of branch slice-18e was clean at f20f08132be34a25ca76490e3140e1dacbbb95c1, tree 0b05f8b296b8d1052ec2a6f2c93bc742a87aa73a. The DONE SHA-256 is 75a9dc306efa6e42629c54acc14f09c0f59a56b4f38052a3b5f9040ba3975d76, exactly matching the reopened dispatch record; dispatch base_commit and slice match the candidate. The reopened freeze names the same head/tree and manifest 4164495a4fab8ebdcd374b3dff6039e016ffc85ea681f38564e3fa26744da2c5. All 439 listed entries matched mode, type, object id and blob SHA-256 (zero mismatches). git ls-tree -r has 487 entries; the precisely 48-entry difference is entirely internal charter/.freeze/** governance records excluded from the portable manifest, with zero non-freeze missing or extra paths.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "BOUNDED SCOPE PASS. git diff --check cff506f..f20f081 exited 0. Its 14 paths are the DONE list, holden-cli source/tests, the charter-only adapter, the embed report, granted fixtures/clients files and xtask/src/gate.rs. The holden-embed delta only records host-relative removed files and the proposal's owner command in EmbedReport. The holden-cognition delta is confined to charter_promises and its Proofs-block state. The xtask delta only adds slice-18e and changes SLICES from 23 to 24. No ungranted production path changed.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 NEXT-STEPS RED/GREEN PASS ON BOAT. Dropping the next-steps heading made the real CLI assertion fail on the missing exact stdout suffix: exit 101 in 47s. Exact revert was diff-clean; tests::successful_embed_prints_next_steps_from_promise_proposals then passed 1/1: exit 0 in 44s. The pin reads owner_command from the actual promise-proposals.json, requires holden approve --host ., the OWNER ONLY marker, and holden agent serve --host . at stdout's end. Both supplied gate records and the independent VM record carry this test [ok].",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 SCOPED-HELP AND BARE-INVOCATION RED/GREEN PASS ON BOAT. Printing global USAGE for every --help made tests::per_command_help_prints_only_that_command fail with 16 command blocks instead of one: exit 101 in 31s. Exact revert was diff-clean; that test passed all 16 real command forms and tests::bare_holden_prints_global_usage_exits_2 independently passed: exits 0,0 in 31s combined. The latter pins global usage, empty stdout, exit 2 and absence of unknown command. All three records carry both names [ok].",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 PRIVATE-ROOT-RELATIVE LEDGER RED/GREEN PASS ON BOAT. Replacing ledger_display_path with the absolute ledger.path() made the real signed-ledger test fail on its relative-path assertion: exit 101 in 29s. Exact revert was diff-clean; tests::ledger_paths_are_relative_to_the_private_root passed 1/1: exit 0 in 30s. The test invokes BOTH ledger verify and ledger telemetry, requires the exact db/ledger.sqlite line and scans all stdout for the sentinel machine root. All three records carry the proof [ok].",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 REMOVED-FILE REPORT RED/GREEN PASS ON BOAT. Suppressing removed-file lines while retaining deletion made tests::removed_files_print_lines_and_the_report_counts_them fail because the before/after directory difference had no removed line: exit 101 in 41s. Exact revert was diff-clean; the test passed 1/1: exit 0 in 41s. It pins the host-relative stale-cognition line and derives the written/unchanged counts from actual wrote:/same: lines before requiring the final written/unchanged/removed trio. All three records carry the proof [ok].",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P5 NO-CANDIDATE REASON RED/GREEN PASS ON BOAT. Disabling no-candidates lines made the exact charter/INTENT.md reason assertion fail: exit 101 in 30s. Exact revert was diff-clean; tests::source_without_candidates_is_named_with_reason passed 1/1: exit 0 in 30s. The output names INTENT and states that it contains goals rather than numbered proofs, with candidates coming from DONE proofs and GUARANTEES. All three records carry the proof [ok].",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P6 PROOFS-BLOCK RED/GREEN + 18B COMPATIBILITY PASS ON BOAT. A compiling mutant accepting numbered lines whenever done_proof was true made tests::only_the_proofs_block_yields_charter_candidates fail its eight-candidate assertion: exit 101 in 31s. Exact revert was diff-clean; the P6 test and the pre-existing tests::charter_yields_numbered_proofs_and_guarantees both passed: exits 0,0 in 31s combined. The eight expected proof/guarantee spans remain byte-for-byte pinned; P9 Steps, P10 Gates and P11 Out-of-scope decoys are each explicitly excluded. An earlier syntactically invalid P6 experiment was discarded and is not counted as teeth. All three records carry both green names.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "INDEPENDENT LINUX VM GATE + REAL REPOSITORY PASS. From the clean QA clone, the Mac-side driver targeted Boat with the configured VM address and VM user under the 2700s outer alarm; exit 0. Record 20261116T011151Z-linux-vm.json names f20f081, a clean tree, green_with_declared_limitations, 212 passed/0 failed/0 ignored, and only cargo_deny_absent, cargo_audit_absent and charter_absent covered by the host guard. Its real-repository names w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples are all [ok]. Directly invoking the driver on Boat itself could not resolve/route the configured VM address or authenticate loopback; those two connection attempts ran no tests and are not represented as green evidence.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "P7 EXACT PLATFORM PARITY PASS. Reopened orchestrator records 20261116T001819Z-macos.json (green, 213/0/0) and 20261116T001802Z-linux-vm.json (green_with_declared_limitations, 212/0/0), plus independent 20261116T011151Z-linux-vm.json (same result/counts). The two Linux name sets are identical (diff exit 0). Against macOS, comm found exactly four host-only names: migrate_removes_legacy_key_after_verify, readme_install_covers_holden_mcp, readme_quickstart_leads_to_green_check and written_mcp_json_drives_w1_through_w4; and exactly three Linux-only names: non_tty_caller_cannot_approve, seal::linux argon2id roundtrip and unseal tty. All seven P1-P6 test names are [ok] in all three records; no new platform-conditional test appears.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "SAFETY AND CLEANLINESS PASS. Canonical charter build 2775000 safety at the candidate exited 0 in 1s: 487 tracked files, zero findings refused and zero allowed; only existing unmatched-allow warnings appeared. Independent scanning of added scoped lines found no home path, account handle or private host identifier. Every effective mutant used CARGO_TARGET_DIR=/tmp/holden-s18e-mutant on Boat, was exactly reverted, and had that literal target removed before the green rerun and again afterward. The Boat QA clone ended diff-clean; neither DEV worktree nor the concurrent 18d lane was modified. No mutant or candidate source change was committed.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "CANONICAL RECEIPT VALIDATION PASS WITH RECORDED OWNER SEAT LIFT. The required canonical charter build 2775000 receipt command, with --allow-same-family qa/slice-18e/mission-qa-seat-applicability.md, exited 0. The applicability note quotes the owner decision that from slice-7 QA on MiniMax and DeepSeek are the different-family QA/worker seats; it has no approved_by field and grants no new bar. The unlifted family-table limitation remains recorded as D-QA-TOOL-2 and is not hidden.",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D-QA-TOOL-2",
      "severity": "info",
      "status": "open",
      "description": "Canonical charter build 2775000 omits MiniMax from its built-in model-family table. The owner-sanctioned same-family lift used by prior accepted mission receipts is therefore required; this is a validator/tool-table gap, not a candidate defect."
    }
  ],
  "new_defects": [],
  "proofs_reopened": [
    "P1 Boat mutant RED exit 101 in 47s; exact-revert GREEN exit 0 in 44s; macOS and both Linux records [ok]",
    "P2 Boat global-help mutant RED exit 101 in 31s; exact-revert per-command and bare-invocation GREEN exits 0,0 in 31s; all records [ok]",
    "P3 Boat absolute-path mutant RED exit 101 in 29s; exact-revert GREEN exit 0 in 30s; both ledger commands scanned; all records [ok]",
    "P4 Boat missing-removed-line mutant RED exit 101 in 41s; exact-revert GREEN exit 0 in 41s; all records [ok]",
    "P5 Boat dropped-no-candidates-line mutant RED exit 101 in 30s; exact-revert GREEN exit 0 in 30s; all records [ok]",
    "P6 Boat compiling decoy-acceptance mutant RED exit 101 in 31s; exact-revert P6 and slice-18b compatibility GREEN exits 0,0 in 31s; all records [ok]",
    "P7 independent VM gate exit 0, 212/0/0; Linux name sets identical and macOS/Linux asymmetry exactly the pre-existing four/three names",
    "Real-repository regression names all [ok] in the independent VM gate; canonical safety exit 0 and canonical receipt validation exit 0"
  ]
}
