{
  "schema_version": "charter.receipt/1",
  "slice": "slice-13",
  "verifies": "slice-13 QA attempt 2: frozen identity and scope, all five prose-discovery proofs and fail-without mutants, real-repository regression, host/Linux records, resolved attempt-1 strict-reopen defect, canonical charter strict gate and guard",
  "candidate_commit": "4df676d7c002f0f6f742e478f2ad003348e13cd8",
  "frozen": {
    "manifest_sha256": "076d86b40058f5765e4dd8f11ce12ead63b24638ced94f69cb3706e27e96f2d3",
    "head": "4df676d7c002f0f6f742e478f2ad003348e13cd8"
  },
  "dispatch": {
    "base_commit": "7b615a1fcd526270c30362b7ee25f04b3272cedd",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at 4df676d7c002f0f6f742e478f2ad003348e13cd8 with tree 98f8cc941210cbee9d1fae9118cee37761e76654. The independently reopened charter.freeze/2 names that head/tree, manifest 076d86b40058f5765e4dd8f11ce12ead63b24638ced94f69cb3706e27e96f2d3 and 350 entries. Independent comparison of every path's Git object id, mode, type and SHA-256 content digest exited 0. DONE sha256 and dispatch done_sha256 both equal f2e0fdcf6fcc6cc49c31b8d26cad5f5834074f30975f3570affae1746da1ea45.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "SCOPE PASS. The full 0170606ae96905bec22b51ad308f29cbb5873cff..4df676d7c002f0f6f742e478f2ad003348e13cd8 diff touches exactly charter/DONE/slice-13.md, two holden-cognition sources, three holden-embed sources plus its integration test, four synthetic prose fixtures, and xtask/src/gate.rs. No path is outside the grant; workflow*.rs is untouched. The xtask delta only adds slice-13 and changes the SLICES length from 16 to 17. git diff --check exited 2 solely for the two intentional trailing spaces in prose-markdown/README.md, which P5 explicitly requires to survive extraction; no other whitespace finding exists.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 CHARTER-PROSE RED/GREEN. Fully qualified holden-cognition tests::prose_charter_yields_verbatim_candidates passed 1/1 (exit 0, 6.60s wall). Removing the prose fallback for marker-only extraction made it RED at DiscoveryKind Explicit versus Prose (exit 101, 6.07s). Exact restore plus git diff --exit-code was clean and the test passed 1/1 again (exit 0, 5.61s). The synthetic prose-charter fixture has no PROMISE marker; the proof checks nonzero byte offsets and verifies every source span. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 MARKDOWN-PROSE RED/GREEN. tests::prose_markdown_yields_candidates_and_names_markdown passed 1/1 (exit 0, 5.92s wall), requiring AdapterKind::Markdown and as_str() == markdown. The same marker-only mutant made it RED at Explicit versus Prose (exit 101, 5.93s). Exact restore and clean-diff check preceded the restored GREEN 1/1 (exit 0, 5.39s). The synthetic prose-markdown fixture has no PROMISE marker and its detection report names markdown. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 AWAITING-OWNER RED/GREEN. holden-embed tests::embed_completes_awaiting_owner_approval passed 1/1 (exit 0, 47.09s wall). It observes the proposal file, markdown detection report, MCP connection and all three /holden command files, no approvals, no plans and zero check runs. A product mutant that agent-signed and admitted unapproved cognitions prepared a plan and made the test RED at prepared.plans.is_empty (exit 101, 46.30s). Exact restore was clean and the test passed 1/1 (exit 0, 46.63s). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 APPROVAL-TO-W2 RED/GREEN. owner_approval_enables_w2_on_approved_promise passed 1/1 on macOS with genuine PTY approval and a traced JIT run (exit 0, 46.72s wall). Removing verify_promise_proposals allowed the paraphrase approval and made the test RED at its refusal assertion (exit 101, 47.05s). Exact restore was clean and the test passed 1/1 (exit 0, 47.97s). The supplied Linux round-2 record marks this test [ok]. The reopened round-1 record has exactly 180 passes and this one [FAILED]; source comparison confirms the repair is the refusal-aware PTY helper: the product refuses the paraphrase before key creation and therefore emits no passphrase prompts, which the old helper incorrectly awaited. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P5 BYTE-EXACT RED/GREEN. tests::proposals_are_byte_exact_substrings passed 1/1 (exit 0, 5.71s wall). Trimming paragraph trailing whitespace before Promise::slice made it RED at trailing source spaces must survive proposal construction (exit 101, 5.18s). Exact restore was clean and the test passed 1/1 (exit 0, 5.78s), checking text equals source[start..end], end-start equals text byte length, repeated spaces, an interior newline and trailing spaces. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "REAL-REPOSITORY REGRESSION PASS. Against the generic private real-repo seed, HOLDEN_E2E_REAL_REPO=<private seed> with an isolated target and timeout -k 10 900 ran the four real_repo tests serially: exit 0, 4 passed, 0 failed, 0 ignored, 208.51s test time and 227.65s wall. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed. Source inspection confirms each run uses git clone --no-local; the seed was never written.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "HOST/VM EVIDENCE + EXACT ASYMMETRY PASS. Reopened host record 20261108T132725Z-macos.json: candidate commit, result green, 179 passed/0 failed/0 ignored, 179 unique names, guard pass. Reopened Linux record 20261108T134233Z-linux-vm.json: same commit, tree_clean true, green_with_declared_limitations, 181 passed/0 failed/0 ignored, 181 unique names. Independent sorted comm shows exactly three VM-only names (non_tty_caller_cannot_approve, unseal_requires_interactive_tty, sealed_roundtrip_uses_argon2id_and_xchacha20poly1305) and one host-only name (readme_quickstart_leads_to_green_check). VM-only cargo_deny_absent, cargo_audit_absent and charter_absent are covered by the passing host guard.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "CHARTER STRICT + GUARD PASS. In the restored clean frozen clone, the canonical pinned build b7a513b check --strict exited 0 with charter: ok and only standing cost-row warnings; every slice-13 DONE proof now has its explicit reopen-and-compare line. Under set -o pipefail, standalone cargo xtask guard with its own target exited 0 and verdict Pass: languages, paths, identifiers, licenses, cargo-deny, cargo-audit and charter_safety all passed. No candidate file was modified by QA.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "ATTEMPT-1 STRICT-REOPEN DEFECT RESOLVED. Attempt 1 was rejected only because the five DONE proofs lacked their reopen steps, causing canonical check --strict to exit 1. The amended DONE list at the attempt-2 tip has an explicit fresh-clone/macOS plus Linux-record reopen-and-compare step for each proof; the independently executed strict command now exits 0. The attempt-2 product reruns above are on the same code with only the governance markdown changed. No waiver or weakened check was used.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "Receipt validated with the canonical charter build (commit b7a513b, the project's pinned release build): charter receipt --verify-tree <clone> <receipt> exit 0, verdict accepted",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D-S13-QA1-STRICT-REOPEN",
      "severity": "info",
      "status": "resolved",
      "description": "Resolved attempt-1 governance defect: all five slice-13 DONE proofs lacked explicit reopen steps and canonical strict check exited 1. The amended dispatched DONE list now contains each reopen-and-compare step and canonical strict check exits 0; no product defect remains."
    }
  ],
  "new_defects": [
    {
      "id": "D-S13-QA1-STRICT-REOPEN",
      "severity": "info",
      "status": "resolved",
      "description": "Historical attempt-1 strict-reopen defect is resolved by the amended DONE list and independently observed canonical check --strict exit 0 at the attempt-2 tip."
    }
  ],
  "proofs_reopened": [
    "P1 executed 1/1; marker-only mutant RED exit 101 in 6.07s, exact revert clean, GREEN 1/1 in 5.61s; Linux gate name [ok]",
    "P2 executed 1/1; marker-only mutant RED exit 101 in 5.93s, exact revert clean, GREEN 1/1 in 5.39s; Linux gate name [ok]",
    "P3 executed 1/1; unapproved-admission/check-preparation mutant RED exit 101 in 46.30s, exact revert clean, GREEN 1/1 in 46.63s; Linux gate name [ok]",
    "P4 executed 1/1 on macOS and Linux executed [ok]; proposal-verification-removal paraphrase mutant RED exit 101 in 47.05s, exact revert clean, GREEN 1/1 in 47.97s; real-repository regression 4/4; round-1 harness RED record reopened",
    "P5 executed 1/1; trailing-whitespace-trimming mutant RED exit 101 in 5.18s, exact revert clean, GREEN 1/1 in 5.78s; Linux gate name [ok]",
    "Canonical charter check --strict executed exit 0 and standalone guard executed exit 0 with verdict Pass; resolved attempt-1 strict-reopen defect reopened and compared"
  ]
}
