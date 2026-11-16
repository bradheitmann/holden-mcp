{
  "schema_version": "charter.receipt/1",
  "slice": "slice-18g",
  "verifies": "slice-18g independent adversarial QA: DONE and frozen identity, bounded scope, whole-item extraction P1-P7 with compiling fail-without mutants, updated 18f and older compatibility pins, three exact-commit read-only client cross-checks with independent per-file counts and cognition provenance, independent Linux VM gate and exact platform parity, canonical safety and receipt validation",
  "candidate_commit": "8d9f96e1a01363562a5e38976c6b1473f6be1a37",
  "frozen": {
    "manifest_sha256": "1dc9957236382efe99afef65e857ba56d213f93af2275e9dadcabf6675aece95",
    "head": "8d9f96e1a01363562a5e38976c6b1473f6be1a37"
  },
  "dispatch": {
    "base_commit": "8d9f96e1a01363562a5e38976c6b1473f6be1a37",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "DONE AND FROZEN IDENTITY PASS. A fresh clone was clean at 8d9f96e1a01363562a5e38976c6b1473f6be1a37, tree 329eba8f449269d21f9a97a49769bfc59dcfa7e8. The actual DONE SHA-256 is 9ee9c19017ef05901b01652fd0ad98688c9c90e6e64160f23c1dccd68ed391e8, exactly matching the reopened dispatch record; dispatch base and slice match the candidate. The assignment's 117b8ead value was stale, not a mismatch in the candidate records. The reopened freeze names the same commit/tree and manifest 1dc9957236382efe99afef65e857ba56d213f93af2275e9dadcabf6675aece95. All 466 entries matched mode, type, object id and path after the established exclusion of generated charter/.freeze/** records; git ls-tree has exactly 466 such entries.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "BOUNDED SCOPE AND OLDER-FIXTURE COMPATIBILITY PASS. git diff --check 073ceb8..8d9f96e exited 0. The adapter delta is confined to charter DONE extraction; the new integration tests and wrapped-charter fixtures are granted. xtask changes only SLICES 25 to 26 and adds slice-18g. The DONE rewrite and dispatch-graph rev-29 change are the sanctioned contract update. The requested diff over fixtures/clients/charter, prose-charter, numbered-charter and heading-charter is EMPTY. Independent Boat compatibility runs: 18b charter_yields_numbered_proofs_and_guarantees exit 0 in 37s; 18e only_the_proofs_block_yields_charter_candidates exit 0 in 36s; all four 18f heading tests exit 0 in 5s.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 WHOLE-ITEM SPAN RED/GREEN PASS ON BOAT. Disabling continuation ownership made the actual fixture return only its first line: exit 101 in 9s, with the expected multiline text shown as the missing right-hand value. Exact revert was diff-clean; wrapped_items_are_one_candidate_with_a_covering_span passed 1/1: exit 0 in 5s. The GREEN pin requires verbatim source[start..end], covering spans and Promise::verify_against, stops before sub-items and decoys, and preserves separate following items. All three gate records carry P1 [ok].",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 BOLD-MARKER RED/GREEN PASS ON BOAT. A compiling mutant dropping the leading bold markers made the source-backed bold candidate unfindable: exit 101 in 5s. Exact revert was diff-clean; bold_names_keep_their_markers passed 1/1: exit 0 in 5s. An earlier syntactically invalid experiment exited 101 during compilation and was discarded, not counted as teeth. All three records carry P2 [ok].",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 BOTH REGRESSION TEETH PASS ON BOAT. Restoring one-line extraction made single_line_candidates_are_unchanged_across_fixtures RED exit 101 in 5s and the updated 18f decoy_sections_stay_excluded_and_older_fixtures_hold RED exit 101 in 5s, both missing the now-required whole wrapped item. Exact-revert GREEN runs were exits 0,0 in 5s,5s. Separately, truncating every item's final byte altered an ordinary single-line candidate and made the older-fixture pin RED exit 101 in 5s; exact revert was GREEN exit 0 in 5s. The green pin checks all 22 expected older candidates, exact spans and content-derived ids.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 LETTERED-NUMBER RED/GREEN PASS ON BOAT. Requiring all-digit markers removed 4b and 4c from the exact candidate vector: exit 101 in 7s. Exact revert was diff-clean; lettered_numbers_are_candidates_and_do_not_end_the_block passed 1/1 with 4, 4b, 4c and following 5: exit 0 in 6s. The live CLIENT-PLANGRAPH target independently confirms the corresponding 4/7 gap is closed at 7/7. All three records carry P4 [ok].",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P5 HEADING-BLOCK RED/GREEN PASS ON BOAT. Closing a proof-heading block at every non-proof heading left only the item before the first lower-level subheading: exit 101 in 5s. Exact revert was diff-clean; heading_blocks_survive_prose_subheadings_and_fences passed 1/1: exit 0 in 5s. Its exact three-item pin also rejects fenced, table and gate decoys and verifies every span. All three records carry P5 [ok].",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P6 LABEL-BLOCK RED/GREEN PASS ON BOAT. Closing a label block at leading top-level prose removed all three expected candidates, including the wrapped-label case: exit 101 in 5s. Exact revert was diff-clean; label_blocks_tolerate_leading_prose_and_still_exclude_decoys passed 1/1: exit 0 in 5s. The same pin rejects trailing narrative and gate decoys. All three records carry P6 [ok].",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P7 UNLABELED-TITLE RED/GREEN PASS ON BOAT. Disabling title-block opening removed both expected candidates: exit 101 in 5s. Exact revert was diff-clean; unlabeled_title_lists_are_proofs_other_headings_are_not passed 1/1: exit 0 in 4s. The pin rejects numbered Gates, Schedule and Landing decoys. All three records carry P7 [ok].",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "P8 READ-ONLY THREE-CLIENT CROSS-CHECK COMPLETE. Exact commits and adapter/independent/cognition totals are CLIENT-WAVE-0 428dc85b6d40d6de08902e146bc6659009e755cb: 1716/1716/879; CLIENT-ODIN 633aef4c40c3f20ad15f6eb856ee9a06da82736c: 170/170/108; CLIENT-PLANGRAPH 5fe216336b77d082d8421f8a71757f0abac4ae1d: 272/272/269. Real-adapter and independent normalized per-file streams are identical, so every differing-file list is empty; the known cs-private-root-v1-qa.md case is now 7/7. Each host's own proposal is AWAITING_OWNER_APPROVAL and its persisted candidate total equals its cognition files (879,108,269), evidencing earlier proposal waves under older extraction rules and, for wave 0 and plangraph, older source snapshots rather than a live-candidate mismatch. CLIENT-ODIN's source and host commits are identical, directly isolating the extraction-rule difference. The full artifact is qa/slice-18g/client-crosscheck-8d9f96e.md. No client or host was written.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "P9 INDEPENDENT LINUX VM GATE AND EXACT PARITY PASS. The Boat driver was FREE before dispatch. The bounded independent VM gate exited 0 in 358s at the exact candidate with a clean tree: record 20261116T074817Z-linux-vm.json, green_with_declared_limitations, 223 passed/0 failed/0 ignored; only cargo_deny_absent, cargo_audit_absent and charter_absent covered by the host guard. Reopened orchestrator records are 20261116T065829Z-macos.json (green, 224/0/0; tests step 2011552 ms) and 20261116T065754Z-linux-vm.json (green_with_declared_limitations, 223/0/0). The two Linux name sets are identical. Against macOS there are 220 shared names plus exactly four pre-existing macOS-only names (migrate_removes_legacy_key_after_verify, readme_install_covers_holden_mcp, readme_quickstart_leads_to_green_check, written_mcp_json_drives_w1_through_w4) and three pre-existing Linux-only names (non_tty_caller_cannot_approve, sealed_roundtrip_uses_argon2id_and_xchacha20poly1305, unseal_requires_interactive_tty). All seven P1-P7 names are [ok] in all three records; no new platform-conditional test appears.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "SAFETY, CLEANLINESS AND CANONICAL RECEIPT VALIDATION PASS. Canonical charter build 2775000 safety at the candidate exited 0 in under 1s: 518 tracked files, zero findings refused and zero allowed; only existing unmatched-allow warnings appeared. Independent scanning of changed adapter/test/fixture/xtask/DONE lines found no home path, account handle or private client identifier. Every effective Boat mutant used its own CARGO_TARGET_DIR, was exactly reverted diff-clean and had that literal target removed after RED and GREEN; compatibility targets were likewise removed. No production socket, other lane, client repository or host state was modified, and no candidate change was committed or pushed. Canonical receipt validation with the owner-sanctioned same-family applicability lift exited 0; the family-table limitation remains D-QA-TOOL-2.",
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
    "P1 Boat continuation-removal mutant RED exit 101 in 9s; exact-revert GREEN exit 0 in 5s; all gate records [ok]",
    "P2 valid bold-marker mutant RED exit 101 in 5s; exact-revert GREEN exit 0 in 5s; compile-only experiment discarded",
    "P3 old truncated behavior RED exits 101,101 in 5s,5s on older and updated-18f pins; GREEN exits 0,0 in 5s,5s; separate single-line truncation RED 101 in 5s and GREEN 0 in 5s",
    "P4 lettered-number rejection RED exit 101 in 7s; exact-revert GREEN exit 0 in 6s; live target 7/7",
    "P5 lower-level-heading termination RED exit 101 in 5s; exact-revert GREEN exit 0 in 5s",
    "P6 leading-prose termination RED exit 101 in 5s; exact-revert GREEN exit 0 in 5s",
    "P7 title-list removal RED exit 101 in 5s; exact-revert GREEN exit 0 in 4s",
    "P8 exact-commit read-only totals 1716/1716/879, 170/170/108 and 272/272/269; zero differing files; proposal/cognition explanation evidenced from host state",
    "P9 independent VM gate exit 0 in 358s, 223/0/0; Linux name sets identical and macOS/Linux asymmetry exactly four/three pre-existing names",
    "18b/18e/18f independent compatibility GREEN exits 0,0,0 in 37s,36s,5s",
    "Canonical safety exit 0 and canonical receipt validation exit 0"
  ]
}
