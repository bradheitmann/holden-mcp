{
  "schema_version": "charter.receipt/1",
  "slice": "slice-10",
  "verifies": "slice-10 QA attempt 3 (final under L-QA): D1/D2 repair, P1-P4, host and Linux gates",
  "candidate_commit": "983c2e58bb21decf0ee3ece6588d5fdb27fe05f8",
  "frozen": {
    "manifest_sha256": "6238ba412b9d3bb830d62dcac32b977cf54585069bfb11de4d1c1c3a7c2bb638",
    "head": "983c2e58bb21decf0ee3ece6588d5fdb27fe05f8"
  },
  "dispatch": {
    "base_commit": "80d73a430e5762982720f65d59971bc18c13530e",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "Frozen identity reopened in a fresh detached scratch clone: HEAD and tree-clean status matched 983c2e58bb21decf0ee3ece6588d5fdb27fe05f8. charter.freeze/2 names the same head, manifest 6238ba412b9d3bb830d62dcac32b977cf54585069bfb11de4d1c1c3a7c2bb638 and 316 entries. Every one of the 316 recorded Git object IDs and SHA-256 content digests matched the checked-out HEAD. DONE sha256 remains 98c46ce77e00a096a812e5918dd251e8f1254209b1ad5d8e39d6bc74c7172aba; no proof growth.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "D1 CLOSED — scenario audit: relay_idle_deadline_ends_held_stdin_session retains ChildStdin in held_stdin until after relay exit; it performs real initialize, initialized and tools/call brief exchanges, then writes nothing and does not close or cancel stdin. Its only normal exit trigger is the idle deadline. The focused unmodified run passed 1/1; relay exit code 1 and stderr named relay idle exchange, and a fresh UnixStream connection to the on-demand server socket was refused after relay exit. The broader focused relay run passed all 5 selected tests.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "D1 CLOSED — decisive fail-without: in a separate scratch clone the only source change replaced runtime.shutdown_timeout(REAP_DEADLINE) with drop(runtime). The fully qualified new test went RED with cargo exit 101 at distribution.rs:414, 'typed deadline: held-stdin relay did not exit after idle shutdown', proving the test's own bound fired on the hung live-stdin runtime drop. The exact test was run, not a zero-test filter. After reverting that single line, the same fully qualified test went GREEN 1/1 and the mutant clone was clean.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "Knob production-safety audit passed. HOLDEN_RELAY_IDLE_DEADLINE_MS is read only inside #[cfg(debug_assertions)] in idle_deadline_limit; non-Unicode, unparsable, negative and zero values fall through, and only a parsed u64 greater than zero can replace the deadline. The fallback remains const IDLE_DEADLINE = Duration::from_secs(120). An additional optimized release run with the test's 1000 ms setting did not exit at the accelerated deadline: the test's own 30-second held-stdin deadline fired at line 414 as expected for release semantics. Thus the debug knob cannot weaken the production 120-second default.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "Relay fixes re-audited: runtime.shutdown_timeout(REAP_DEADLINE) occurs after block_on(relay), not a plain Runtime drop with a live stdin reader. relay awaits ChildGuard::shutdown before returning, with bounded kill/wait; Drop starts kill and uses a separate reaper thread. Every read and every partial write is idle-bounded, completed bytes in either direction refresh the shared activity timestamp, EOF shuts down the socket writer and cleanly joins the response direction, and no connection-age timeout exists. relay_exits_when_client_closes_stdin passed; relay_session_survives_long_active_session passed after 140255 ms (>130 s), disproving connection-age expiry. The old EOF-only plain-drop mutant is no longer the decisive control; the new held-stdin test supplies that non-vacuous control. Other relay scenarios and assertions were unchanged by the repair.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P1 reopened in the fresh full host gate: init_is_idempotent, init_refuses_world_readable_paths and init_never_prints_secrets each [ok]. The same three names are [ok] in the executed Linux VM record.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P2 reopened: mcp_stdio_relay_spawns_server_on_demand and mcp_stdio_relay_carries_implementer_role each [ok] in both platform records. The focused real-process run also passed these plus relay_exits_when_client_closes_stdin, relay_idle_deadline_ends_held_stdin_session and relay_session_survives_long_active_session; D1's plain-drop mutant is RED/GREEN as recorded in C03.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P3 reopened in the fresh full host gate: embed_configs_validate_per_harness and embed_is_idempotent each [ok]. Both identical names are [ok] in the Linux VM record.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P4 reopened in the fresh full host gate: holden_command_maps_to_workflows_only [ok], with the identical named test [ok] in the Linux VM record.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "D2 CLOSED / merge scope audit: git diff 51b9a65..983c2e5 contains exactly crates/holden-cli/src/mcp_cmd.rs, crates/holden-cli/tests/distribution.rs and charter/STATE.md (121 insertions, 4 deletions); distribution.rs is additive. Main merge 322074a brings in STATE refreshed at 2026-11-08T02:13:48Z to post-slice-9 main HEAD 000f3d8d650c37852a221597bf0ea2f01b65fd48, replacing the stale pre-slice-9 be7ea076 header. git diff --check passed. charter check --strict on the isolated tip exited 0 with charter: ok and only the standing accepted-slice cost-row warnings.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "Fresh independent host gate executed from the detached clean tip with its own outer CARGO_TARGET_DIR=/tmp/holden-mcp-qa10c-target-gate-final: cargo xtask gate slice-10 exited 0. Reopened run record 20261108T030728Z-macos.json: commit 983c2e58bb21decf0ee3ece6588d5fdb27fe05f8, tree_clean true, result green, guard_verdict pass; fmt, clippy, tests and guard all pass; 160 passed, 0 failed, 0 ignored; exactly 160 unique recorded names, every name [ok].",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "Standalone cargo xtask guard at the isolated tip with its own CARGO_TARGET_DIR=/tmp/holden-mcp-qa10c-target-guard exited 0: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all pass; verdict Pass. No guard waiver or weakening was introduced.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "Executed Linux VM evidence reopened without new VM contact: 20261108T025950Z-linux-vm.json records commit 983c2e58bb21decf0ee3ece6588d5fdb27fe05f8, tree_clean true, result green_with_declared_limitations, 160 passed, 0 failed, 0 ignored, all 160 names [ok]. Its test names are identical to both the supplied host record 20261108T024645Z-macos.json and this QA run 20261108T030728Z-macos.json; counts match 160/160. Declared VM-only cargo_deny_absent, cargo_audit_absent and charter_absent limitations are covered by the passing host guard. VM gate exit is recorded successful by the gate result.",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": [
    "P1: init_is_idempotent, init_refuses_world_readable_paths, init_never_prints_secrets [ok]; host executed; Linux executed (160/160 VM gate)",
    "P2: on-demand spawn and implementer-role tests [ok]; real held-stdin idle shutdown [ok]; plain-Runtime-drop mutant RED then revert GREEN; EOF and 140255 ms active-session tests [ok]; host and Linux executed",
    "P3: embed_configs_validate_per_harness and embed_is_idempotent [ok]; host and Linux executed",
    "P4: holden_command_maps_to_workflows_only [ok]; host and Linux executed"
  ]
}
