{
  "schema_version": "charter.receipt/1",
  "slice": "slice-18f",
  "verifies": "slice-18f independent adversarial QA: DONE and frozen identity, bounded scope, Markdown proof-heading and bullet proofs P1-P4 with compiling fail-without mutants, bold-named source spans, slice-18b/18e compatibility, a combined real-form repro, three read-only client-clone cross-checks, independent Linux VM gate and exact platform parity, canonical safety and receipt validation",
  "candidate_commit": "cccb2caaf3bb97e8b2ea22f779254f97162ca9a4",
  "frozen": {
    "manifest_sha256": "0e8b25ff329661cd3a6a09f54df27f8fd86f5249f9d36e651f66a69cb589c5de",
    "head": "cccb2caaf3bb97e8b2ea22f779254f97162ca9a4"
  },
  "dispatch": {
    "base_commit": "cccb2caaf3bb97e8b2ea22f779254f97162ca9a4",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "DONE AND FROZEN IDENTITY PASS. A fresh clone was clean at cccb2caaf3bb97e8b2ea22f779254f97162ca9a4, tree 7abbb7110e2f4a99ecdf1535e9cda5f062cb8ba6. DONE SHA-256 ef52e40a75c8fcdd28309b30ab686cf07945a8ec11e8fc1c46f05e196c39cbff exactly matches the reopened dispatch record; dispatch base and slice match the candidate. The reopened freeze names the same commit/tree and manifest 0e8b25ff329661cd3a6a09f54df27f8fd86f5249f9d36e651f66a69cb589c5de. All 453 listed entries matched mode, type and object id with zero mismatches; git ls-tree has exactly 453 non-charter/.freeze entries.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "BOUNDED SCOPE AND OLDER-FIXTURE COMPATIBILITY PASS. git diff --check 84688eb..cccb2ca exited 0. The adapter delta is confined to charter_promises; its new integration test and heading-charter fixtures are granted; xtask only registers slice-18f and changes SLICES 24 to 25. The dispatch-graph revision 26 to 28 delta, including the already-authored slice-18g follow-up, came from the sanctioned main merge rather than DEV scope. The requested diff over fixtures/clients/charter, prose-charter and numbered-charter is EMPTY. holden-cli::tests::charter_yields_numbered_proofs_and_guarantees and holden-cli::tests::only_the_proofs_block_yields_charter_candidates are [ok] in every gate record.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 MARKDOWN PROOF HEADINGS RED/GREEN PASS ON BOAT. Reverting block start to bare Proofs only made the actual heading fixture yield zero of 13 expected candidates: exit 101 in 7s. Exact revert was diff-clean; proof_blocks_start_at_proofs_lines_and_markdown_headings passed 1/1: exit 0 in 5s. Each run used and then literally removed its own mutant target. The bounded macOS focused test also passed all four 18f tests in 45s, and all three gate records carry P1 [ok].",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 TOP-LEVEL BULLETS RED/GREEN PASS ON BOAT. Dropping bulleted from candidate bodies reduced the exact three-item fixture to only its numbered companion: exit 101 in 5s. Exact revert was diff-clean; top_level_bullets_are_candidates_indented_subitems_are_not passed 1/1: exit 0 in 5s. The pin rejects Reopen:, Fail-without mutant:, Test: and indented bullet decoys. All three gate records carry P2 [ok].",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 DECOY EXCLUSION RED/GREEN PASS ON BOAT. Admitting top-level items throughout DONE files changed the older numbered-charter set from eight to eleven, admitting the P9 Steps, P10 Gates and P11 Out-of-scope decoys: exit 101 in 5s. Exact revert was diff-clean; decoy_sections_stay_excluded_and_older_fixtures_hold passed 1/1: exit 0 in 5s. Its eight numbered-charter proof/guarantee expectations and one prose-charter expectation remain exact. All three gate records carry P3 [ok].",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "BOLD-NAMED SPAN AND COMBINED MINIMAL REPRO PASS. bold_named_proof_item_under_a_heading_is_a_candidate pins the complete **bold-name.** candidate, verifies it against source bytes and pins line 4; it passed in the bounded macOS run and every gate record. An independent combined DONE fixture exercising Proofs, Fixed proof groups, Proofs (REV-4), Proof 1 — title, numbered/dash/star items, bold naming, indented sub-items, Gates/Steps/scope decoys and a top-level terminator yielded exactly nine expected candidates at lines 3, 4, 9, 10, 11, 13, 15, 16 and 24, each exactly once with verified source spans and no decoys; runner exit 0 in 35s.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "READ-ONLY THREE-CLIENT CROSS-CHECK COMPLETE. The real adapter and an independently written proof-state counter agree on CLIENT-WAVE-0: 978/978 and CLIENT-ODIN: 108/108 with zero differing files. CLIENT-PLANGRAPH is 269 adapter versus 272 independent solely at charter/DONE/cs-private-root-v1-qa.md (4 versus 7): that file contains 4b. and 4c., intentionally counted by the independent Na. grammar but outside slice-18f's digits-only grammar. The difference also drops following proof 5 because the unrecognised top-level line ends the block. This exact case and its repair are already predeclared in the inherited main plan as slice-18g P4, so it is not a new 18f defect or silently waived mismatch. No client repository was written.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "INDEPENDENT LINUX VM GATE PASS. The direct Boat-side driver could not resolve its configured VM hostname and exited 1 before tests; this connection-only attempt is not counted as green evidence. The established Mac-side driver then targeted Boat from a separate clean frozen clone under the 2700s outer alarm and exited 0 in 356s. Record 20261116T025238Z-linux-vm.json names cccb2ca, a clean tree, green_with_declared_limitations, 216 passed/0 failed/0 ignored, and only cargo_deny_absent, cargo_audit_absent and charter_absent covered by the host guard.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P4 EXACT PLATFORM PARITY PASS. Reopened orchestrator records 20261116T013336Z-macos.json (green, 217/0/0) and 20261116T013320Z-linux-vm.json (green_with_declared_limitations, 216/0/0), plus independent 20261116T025238Z-linux-vm.json (same result/counts). The two Linux name sets are identical (diff empty). Against macOS, comm found exactly the four pre-existing host-only names: migrate_removes_legacy_key_after_verify, readme_install_covers_holden_mcp, readme_quickstart_leads_to_green_check and written_mcp_json_drives_w1_through_w4; and exactly three Linux-only names: non_tty_caller_cannot_approve, sealed_roundtrip_uses_argon2id_and_xchacha20poly1305 and unseal_requires_interactive_tty. All three P1-P3 names and the bold-named test are [ok] in all three records; no new platform-conditional test appears.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "SAFETY AND CLEANLINESS PASS. Canonical charter build 2775000 safety at the candidate exited 0: 503 tracked files, zero findings refused and zero allowed; only existing unmatched-allow warnings appeared. Independent scanning of added adapter/test/fixture lines found no home path, account handle or private host identifier. Every effective Boat mutant used its own CARGO_TARGET_DIR, was exactly reverted diff-clean and had that literal target removed after RED and GREEN. The Mac focused run used a bounded non-production scratch target. No full macOS gate was run by QA, no production socket or other lane was touched, and no source change was committed or pushed.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "CANONICAL RECEIPT VALIDATION PASS WITH RECORDED OWNER SEAT LIFT. The required canonical charter build 2775000 receipt command, with --allow-same-family qa/slice-18f/mission-qa-seat-applicability.md, exited 0. The applicability note quotes the owner decision that from slice-7 QA on MiniMax and DeepSeek are the different-family QA/worker seats; it has no approved_by field and grants no new bar. The unlifted family-table limitation remains D-QA-TOOL-2 and is not hidden.",
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
    "P1 Boat heading-removal mutant RED exit 101 in 7s; exact-revert GREEN exit 0 in 5s; bounded macOS and both Linux records [ok]",
    "P2 Boat bullet-removal mutant RED exit 101 in 5s; exact-revert GREEN exit 0 in 5s; all records [ok]",
    "P3 Boat decoy-admission mutant RED exit 101 in 5s (8 to 11 candidates); exact-revert GREEN exit 0 in 5s; older fixtures byte-unchanged and 18b/18e pins [ok]",
    "Bold-named source text/span/line pin GREEN on bounded macOS and every gate record",
    "Combined real-form repro GREEN exit 0 in 35s: exactly nine expected candidates with source lines and zero decoys",
    "Read-only client totals 978/978, 108/108, 269/272; sole differing file and lettered-number cause recorded and already assigned to slice-18g P4",
    "P4 independent VM gate exit 0 in 356s, 216/0/0; Linux name sets identical and macOS/Linux asymmetry exactly four/three pre-existing names",
    "Canonical safety exit 0 and canonical receipt validation exit 0"
  ]
}
