{
  "schema_version": "charter.receipt/1",
  "slice": "slice-9",
  "verifies": "slice-9",
  "candidate_commit": "162c346d25761957c53189b153556cb12924aa6b",
  "frozen": {
    "manifest_sha256": "c2d6af9887937b3fb6a2d5ebccc6654f1dbbc9c45f73238cde3947c213db88d7",
    "head": "162c346d25761957c53189b153556cb12924aa6b"
  },
  "dispatch": {
    "base_commit": "18c726531d4c0af93a2dd21b0e387de097cafc2f",
    "brief_sha256": "642693147378192bf8157230580ac2f60497fef3c0e52510d61e6f5a2e14d011"
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "Frozen identity at the merged tip: HEAD 162c346d25761957c53189b153556cb12924aa6b; charter.freeze/2 head and commit match; manifest_sha256 c2d6af9887937b3fb6a2d5ebccc6654f1dbbc9c45f73238cde3947c213db88d7; entry_count 308. DONE sha256 remains 642693147378192bf8157230580ac2f60497fef3c0e52510d61e6f5a2e14d011. QA-1 reopened the original frozen identity in a clean detached scratch clone with 290 entries and matching head, commit, manifest and DONE hash.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "P1 host proofs reopened separately: holden-cognition tests::detection_is_deterministic and tests::unknown_process_falls_back_with_typed_limitation each passed 1/1. Adversarial source audit found explicit Charter > DispatchGraph > Markdown precedence, sorted read_dir entries and sorted final document paths, no map-order selection, and AdapterLimitation::UnknownProcess with stable code unknown_process_markdown_fallback rather than a silent pass.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 fail-without mutant in its own scratch clone replaced fixed precedence with first-match HashMap iteration. detection_is_deterministic went RED (left DispatchGraph, right Charter; cargo exit 101); after exact source revert the same named test went GREEN 1/1.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 host proof tests::promise_extraction_is_verbatim_byte_offset passed 1/1 against all three real synthetic client fixture formats. Adversarial audit confirmed Promise spans retain start/end offsets into PlanningDocument.bytes, text equals the exact source slice, double spaces and tabs are exercised, and no normalization or paraphrase occurs.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P2 fail-without mutant normalized promise whitespace in a separate scratch clone. promise_extraction_is_verbatim_byte_offset went RED with QuoteDiffers { start: 95, end: 166 }; after exact revert it went GREEN 1/1.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P3 graph host proof tests::graph_writeback_passes_client_validation passed 1/1. Source audit reopened the real std::process::Command invoking jq -f plans/validate-graph.jq plans/dispatch-graph.json in the copied fixture client; the emitted HOLDOUT depends_on FEATURE-QA and the validator returned valid=true and holdout=1.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P3 charter host proof tests::charter_writeback_never_touches_pinned_lists passed 1/1. The test compares pinned slice-1.md bytes before and after both refused proposal-to-pin and explicit apply; the implementation targets only charter/DONE/TEMPLATE.md. P4 tests::writeback_is_proposal_only_by_default also passed 1/1: default wrote a patch without changing the graph, explicit apply changed the graph, and charter apply cannot select a pinned DONE list.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P3 fail-without mutant changed HOLDOUT depends_on selection from QA to DEV nodes in a separate scratch clone. graph_writeback_passes_client_validation went RED because the real client validator returned valid=false; after exact revert it went GREEN 1/1.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P4 fail-without mutant changed WritebackOptions::default apply from false to true in a separate scratch clone. writeback_is_proposal_only_by_default went RED at !outcome.applied; after exact revert it went GREEN 1/1. No mutant was committed and every mutant clone was clean after revert.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "QA-1 scope diff 18c726531d4c0af93a2dd21b0e387de097cafc2f..a513c8bf90f9d3ada5f42890024bea30b8b61fc5 contains exactly 12 authorized files: DONE/slice-9.md, two holden-cognition source files, eight fixtures/clients files, and xtask/src/gate.rs. The xtask change is only SLICES length 11->12 plus the slice-9 entry. crates/holden-cli has ZERO diff. DONE retains exactly P1-P4 and fail-without mutants matching SLICE-09-DEV acceptance items; no proof growth. The acceptance tip merges accepted main and its guard-only follow-up.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "Final host gate on the clean merged tip: CARGO_TARGET_DIR=/tmp/holden-mcp-accept-s9-target timeout -k 10 2400 cargo xtask gate slice-9. Result green; 149 passed, 0 failed, 0 ignored; all 149 recorded names [ok]; fmt, clippy, tests, guard pass. Run record 20261108T020428Z-macos.json.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "QA-1 charter check --strict exited 0 with charter: ok and pre-existing accepted-slice cost-row warnings. Standalone CARGO_TARGET_DIR=/tmp/holden-mcp-qa9-target-guard cargo xtask guard exited 0: every named guard check passed and verdict Pass. The merged-tip host and VM gates subsequently passed all available guard checks.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "Linux/VM gate executed at merged tip 162c346d25761957c53189b153556cb12924aa6b: result green_with_declared_limitations; 149 passed, 0 failed, 0 ignored; all 149 names [ok] and test names identical to host run 20261108T020428Z-macos.json; pass counts identical (149/149). Declared limitations cargo_deny_absent, cargo_audit_absent and charter_absent are covered by the host guard. VM gate run record 20261108T020701Z-linux-vm.json; inner Linux record 20261108T020554Z-linux.json; VM gate exit code 0.",
      "classification": "executed"
    },
    {
      "id": "C14",
      "description": "The previously deferred Linux/VM evidence is now executed. The acceptance integration merged accepted slices 7-8 and the generic-name guard follow-up into slice-9; no slice-9 proof or acceptance bar was weakened.",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": [
    "P1 host: detection_is_deterministic; unknown_process_falls_back_with_typed_limitation; detection-order mutant RED/GREEN; host: executed; Linux: executed (VM gate green)",
    "P2 host: promise_extraction_is_verbatim_byte_offset; whitespace-normalization mutant RED/GREEN; host: executed; Linux: executed (VM gate green)",
    "P3 host: graph_writeback_passes_client_validation; charter_writeback_never_touches_pinned_lists; DEV-dependency mutant RED/GREEN; host: executed; Linux: executed (VM gate green)",
    "P4 host: writeback_is_proposal_only_by_default; apply-by-default mutant RED/GREEN; host: executed; Linux: executed (VM gate green)"
  ]
}
