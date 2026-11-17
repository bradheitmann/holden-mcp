{
  "schema_version": "charter.receipt/1",
  "slice": "slice-18h",
  "verifies": "slice-18h attempt-2 independent adversarial QA at repaired frozen bytes: unchanged DONE/dispatch identity, new freeze and bounded D1 scope, D1 and P1-P7/P6b fail-without teeth on Boat, unchanged older pins, zero-difference per-file three-client cross-checks, derived platform parity, canonical safety and receipt validation",
  "candidate_commit": "a53511f165b0ec3d8d8f1211d3fb0563e9177643",
  "frozen": {
    "manifest_sha256": "10ad7f89e38bd0c11b65e8b464e9fc3db911910782ddac4049ce4b148e08119a",
    "head": "a53511f165b0ec3d8d8f1211d3fb0563e9177643"
  },
  "dispatch": {
    "base_commit": "a53511f165b0ec3d8d8f1211d3fb0563e9177643",
    "brief_sha256": null
  },
  "models": {
    "author": "deepseek-v4.1-flash",
    "verifier": "kimi-k3.1-pro",
    "author_maker": "DeepSeek",
    "verifier_maker": "Moonshot AI",
    "authors": [
      {"model": "deepseek-v4.1-flash", "maker": "DeepSeek", "seat": "DEV-WORKER"}
    ],
    "verifiers": [
      {"model": "kimi-k3.1-pro", "maker": "Moonshot AI", "seat": "FRESH-QA-VALIDATOR"}
    ]
  },
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "IDENTITY PASS. HEAD is a53511f165b0ec3d8d8f1211d3fb0563e9177643 with no tracked modifications; only QA's own untracked artifacts are present. DONE remains byte-identical at SHA-256 4b4da8cfce9d456a41a0ac16b501929b0535643e4ecb2aa735f09cb42ead6e7e, matching dispatch. Dispatch base is a53511f. The new freeze names a53511f and manifest 10ad7f89e38bd0c11b65e8b464e9fc3db911910782ddac4049ce4b148e08119a. All 491 entries matched Git mode/type/object/path and blob SHA-256 after excluding generated charter/.freeze/** records; the filtered git tree also has 491 entries. Tree diff exit 0; hash mismatches 0.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "D1 SCOPE PASS. git diff --name-status a3e062f..a53511f names exactly adapters.rs, charter_blind_spots.rs, embed_cmd.rs, embed_report.rs and the new slice-plain-continuations.md fixture. git diff --check exited 0. The production delta is two condition lines: the adapter requires bulleted.is_some() for short-item rescue, and CLI accounting requires an indented -/* prefix. Legacy fixture diffs are empty and changed-line scanning found no home path, account handle or private client identifier.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "D1 TEETH PASS ON BOAT. Reverting the adapter to any-indented-line rescue made plain_continuations_are_not_bulleted_subitems RED exit 101 in 5s; exact revert was diff-clean and GREEN exit 0 in 4s. The green fixture pins zero candidates, seen 1, taken 0, too_short 1 and outside 0. Reverting CLI accounting to any-indented-line made per_source_report_counts_items_taken_and_left_out RED exit 101 in 44s; exact revert was GREEN exit 0 in 45s. The original P1 true-bulleted-sub-item fixture was independently rerun RED 101/5s and GREEN 0/5s, proving the repair did not drop legitimate claim extension.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P1-P3 RE-PROOF PASS. P1 short-item-drop mutant RED exit 101/5s, exact-revert GREEN 0/5s. P2 altered-long-item mutant RED 101/4s, GREEN 0/5s. P3 admit-short-item mutant RED 101/5s, GREEN 0/5s. Exact source restoration was diff-clean after every round. P1's bold and ordinary short claim spans still verify against source bytes, P2's long text/span/id remains unchanged, and P3 now covers both Tiny and the real plain-continuation shape.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P4/P5 RE-PROOF PASS. P4 disabling empty-label title fallback: RED exit 101/4s, exact-revert GREEN 0/4s. P5 emptying marked-list lookup: RED exit 101/5s, exact-revert GREEN 0/4s. Title fallback, marked proof lists and all decoy exclusions remain pinned.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P6/P6b RE-PROOF PASS. P6 dropping report accounting wordings: RED exit 101/44s, exact-revert GREEN 0/44s. The green test pins seen/taken/too short/outside wordings, numbered items found none taken, no numbered items, and the new plain-continuation line seen 1/taken 0/too_short 1/outside 0. P6b allowing title words to open a block: RED exit 101/5s, exact-revert GREEN 0/4s; four Design bullets stay out and five real proofs remain.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "BACKWARD COMPATIBILITY PASS WITHOUT EDITS. On Boat, 18b+18e charter_promises exited 0 in 155s, 18f charter_proof_headings exited 0 in 4s and 18g charter_whole_items exited 0 in 5s. The older fixture directories have an empty diff against attempt 1 and their original candidate spans remain pinned.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P6c ZERO-DIFFERENCE PER-FILE CROSS-CHECK PASS. Repaired real-adapter/independent comparisons cover CLIENT-WAVE-0 finding 5fb964bad3f27fd7ba29b0f83a78d22a969b2210 (290 files), CLIENT-WAVE-0 current 8a74fb06a7d5dd6e55976a3c2de5a2c9d2b3fd7f (311), CLIENT-ODIN 8c26fb36e7032c32def210be8a68ae3e6577b4a2 (36) and CLIENT-PLANGRAPH 6373fce05c7a4d6d6e5b5c6d81f7e28caecf0ea5 (42). Every file matches in both directions; there are no adapter-greater or independent-greater rows. Attempt-1 p00-depth-oracle-scope.md now matches exactly: independent 0 taken/1 seen; adapter 0 taken/1 seen/1 too_short/0 outside in both wave-0 snapshots. All returned spans verified and no client or host was written.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P7 FOUR-FILE AND REASON-ACCOUNTING PASS. At finding commit 5fb964ba, offline-revalidate.md, calibration-recovery-route.md, p05-source-capture.md and reconciler-candidate-view.md yield 5,6,6,4 in both counters. Per-client totals are finding independent/adapter numbered 1493/1493 with seen 1511, current wave-0 1675/1675 with seen 1738, ODIN 174/174 with seen 177, and PLANGRAPH 283/283 with seen 290. Every seen>taken file and its too_short/outside reason counts is listed in qa/slice-18h/client-crosscheck-a53511f.md.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "P8 DERIVED PARITY PASS; ACTUAL 18H MAC GATE REMAINS PENDING. Reopened VM record 20261117T035627Z-linux.json is plain green at a53511f: 231 passed/0 failed/0 ignored, fmt/clippy/tests/guard pass and cargo-deny/cargo-audit/charter-safety pass. Against 18g-era macOS record 20261116T065829Z-macos.json, macOS-only names are exactly the four pre-existing names and Linux-only names are exactly eight 18h tests (the original seven plus plain_continuations_are_not_bulleted_subitems) plus the three pre-existing Linux names. None of the eight is platform-conditional in source; adding them to the macOS baseline yields the expected 18h name set. This is the assigned parity derivation, not a claim that the pending macOS gate ran.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "SAFETY AND HYGIENE PASS. Canonical c6c9843 safety at a53511f on Boat exited 0 in under 1s: 545 tracked files, zero findings refused and zero allowed. Every mutant used an isolated target/TMPDIR, produced expected RED 101, restored exact source and passed diff-clean GREEN; all per-run targets and scratch were removed by literal path. No Mac test/build/check, client write, host-state write, commit, push or accept-chain step occurred.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "CANONICAL RECEIPT VALIDATION. The re-pinned c6c9843 charter validated this accepted receipt without --allow-same-family, using explicit DeepSeek and Moonshot AI makers and separate DEV-WORKER/FRESH-QA-VALIDATOR seats. Canonical reopen recorded proofs 1-10 against VM run 20261117T035627Z-linux with evidence SHA-256 059f6c573cc5f05f0a9878d13b47c50939c01256a092b153b6d6767cafa23f41, exit 0. Receipt validation then exited 0 with verdict accepted and receipt --verify-tree . exited 0 against the exact a53511f/10ad7f89 freeze. Final-byte validation was rerun after recording these results and again exited 0.",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
}
