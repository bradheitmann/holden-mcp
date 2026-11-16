{
  "schema_version": "charter.receipt/1",
  "slice": "slice-14",
  "verifies": "slice-14 independent adversarial QA: frozen identity and amendment-bounded scope, all three first-class server-run proofs and fail-without mutants, platform parity, real-repository regression, canonical charter strict check and full guard",
  "candidate_commit": "e13757095b3fdda7856ca9c0fa21d61ad118682d",
  "frozen": {
    "manifest_sha256": "9288659838b4873e053e4c642c0533f2e8e6f9d8a5caf5c1b55eb582e58d1b18",
    "head": "e13757095b3fdda7856ca9c0fa21d61ad118682d"
  },
  "dispatch": {
    "base_commit": "b7b753f2260878d37b4cf20108f83321c62f8e77",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at e13757095b3fdda7856ca9c0fa21d61ad118682d with tree d9a75e84ec743309da82932fc5acc5c902657efe. The independently reopened charter.freeze/2 names that head/tree, manifest 9288659838b4873e053e4c642c0533f2e8e6f9d8a5caf5c1b55eb582e58d1b18 and 356 entries. Every listed path's Git object id, mode, type and file SHA-256 was compared and all 356 matched. DONE SHA-256 and the current dispatch done_sha256 both equal 15774970136a711d18804df0bebe3019b27d52463b9e0730465bdf1a73e8caf9; dispatch base matches this receipt.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "SCOPE PASS. The complete 2fca3dc8c17dda8733b2bfe8748584da60d2b627..e13757095b3fdda7856ca9c0fa21d61ad118682d diff touches exactly 14 granted paths: DONE, holden-mcp workflow.rs plus amendment-1 server.rs/ledger.rs, holden-ledger measure.rs/records.rs, holden-embed jit.rs/trace.rs and its real_repo test, holden-cli embed_cmd.rs/main.rs and the new server_runs test, holden-mcp workflows tests, and xtask gate.rs. The xtask change only registers slice-14 and bumps 17 to 18. The server.rs delta passes an optional evaluated commit into W2; ledger.rs retains generated-plan provenance and appends the signed verdict entry; nothing else appears there. CLI changes are run-id trace/replay resolution, real_repo only permits one float ULP across separately compiled codegen units, and git diff --check exits 0.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 COMMIT BINDING RED/GREEN PASS. Fully qualified holden-mcp tests::w2_check_evaluates_the_commit_under_test passed 1/1 on macOS (exit 0, 48.13s wall). The proof checks the caller-named original commit in the response and server record, then HEAD-at-request in both, preserving the same client HEAD and complete history after each run. Both runs go through the real server's verdict path, where replay under the configured evaluator key binds rec.body.evaluated_commit to begin_run. A mutant that made W2 create an empty holden commit before evaluating HEAD went RED at unchanged client HEAD (exit 101, 42.58s wall). Exact restore plus git diff --exit-code was clean and the same test returned GREEN 1/1 (exit 0, 46.05s wall). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 LEDGER TRACE/REPLAY/AUDIT RED/GREEN PASS. holden-cli tests::server_w2_run_traces_replays_and_reveals passed 1/1 on macOS (exit 0, 43.47s wall). For a run composed by MCP holden_check, real holden trace printed verified every link, real holden replay resolved the run id and printed the client evaluated_commit, and ServerReveal::from_reveal_body independently verified audit_reveal's VRF output/proof against plan_commit, evaluated_commit and the epoch key before equality with the signed record reveal. The skip-ledger-append mutant went RED with refused: ledger: the server run's verdict is absent (exit 101, 43.91s wall). Exact restore was clean and the proof returned GREEN 1/1 (exit 0, 44.07s wall). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 EFFECTIVENESS RED/GREEN PASS. holden-mcp tests::w4_counts_server_side_runs passed 1/1 on macOS (exit 0, 45.33s wall). W4 moved from 0 verdicts and no signed weights to 1 verdict with every check catch sample n=1 and a signed weights record, then a second client commit's W2 produced 2 verdicts, every check n=2, a different signed weights at ledger pointer, and a weight basis sample n=2. Removing VERDICT from Measurements::from_entries went RED at the n=1 catch-sample assertion (exit 101, 44.44s wall). Exact restore plus a clean diff returned the proof GREEN 1/1 (exit 0, 45.04s wall). Control verdict hashes are explicitly excluded from the real-verdict path to prevent duplicate trials. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 HOST/LINUX PARITY PASS. Reopened 20261108T150008Z-macos.json: candidate e137570, tree_clean true, green, 182 passed/0 failed/0 ignored, 182 unique all-ok names and guard_verdict pass. Reopened 20261108T151334Z-linux-vm.json without new VM contact: same commit, tree_clean true, green_with_declared_limitations, 184 passed/0 failed/0 ignored, 184 unique all-ok names. Independent sorted comm has exactly three VM-only names (non_tty_caller_cannot_approve, unseal_requires_interactive_tty, sealed_roundtrip_uses_argon2id_and_xchacha20poly1305) and exactly one host-only name (readme_quickstart_leads_to_green_check). P1-P3 are [ok] in both records. The VM guard's only limitations are cargo_deny_absent, cargo_audit_absent and charter_absent, all covered by the independently passing host guard.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "REAL-REPOSITORY REGRESSION PASS. Against the generic private real-repo seed, own-target HOLDEN_E2E_REAL_REPO=<private seed> with timeout -k 10 900 ran the four real_repo tests serially: exit 0, 4 passed, 0 failed, 0 ignored, 214.76s test time and 232.91s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. Source inspection confirms each real test clones the seed afresh using git clone --no-local into an isolated temporary host; no test writes to the sacred seed.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "CHARTER STRICT + GUARD PASS. In the clean frozen clone, the canonical pinned release build b7a513b check --strict exited 0 with charter: ok and only standing accepted-slice cost-row warnings. Under set -o pipefail, standalone cargo xtask guard with its own CARGO_TARGET_DIR exited 0 and verdict Pass: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all passed. Every foreground verification used a 2700-second outer alarm and its own target; no mutant was committed and no candidate file was modified.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "Receipt validated with the canonical charter build (commit b7a513b, the project's pinned release build): charter receipt --verify-tree <frozen clone> <receipt> exit 0, verdict accepted.",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": [
    "P1 host executed independently 1/1; holden-made-check-commit mutant RED exit 101 in 42.58s, exact revert clean, GREEN 1/1 in 46.05s; Linux gate name [ok]",
    "P2 host executed independently 1/1; skip-ledger-append mutant RED exit 101 in 43.91s, exact revert clean, GREEN 1/1 in 44.07s; trace/replay and VRF audit all reopened; Linux gate name [ok]",
    "P3 host executed independently 1/1; verdict-telemetry-omission mutant RED exit 101 in 44.44s, exact revert clean, GREEN 1/1 in 45.04s; Linux gate name [ok]",
    "P4 independent comm compared 182 macOS and 184 Linux unique all-ok names with exactly the four expected platform asymmetries; P1-P3 occur and pass on both",
    "Canonical charter check --strict exit 0, standalone guard exit 0/verdict Pass, and real-repository regression 4/4 reopened"
  ]
}
