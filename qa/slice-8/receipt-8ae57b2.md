{
  "schema_version": "charter.receipt/1",
  "slice": "slice-8",
  "verifies": "slice-8",
  "candidate_commit": "8ae57b2dc927b200b742135f941e5cdb1f8d532e",
  "frozen": {
    "manifest_sha256": "08e6a4bc484d0989ee1f9584917e9456573caaa910c27806630ed56b1bbe7059",
    "head": "8ae57b2dc927b200b742135f941e5cdb1f8d532e"
  },
  "dispatch": {
    "base_commit": "3326c08ebd17ff184f7bb191aab32983694c6841",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "Fresh detached clone at 18c726531d4c0af93a2dd21b0e387de097cafc2f was clean. The supplied charter.freeze/2 record names the same head, kind git-commit-tree, 280 entries and manifest 460876d5446ade3194657a0384bc69d087a0895f781f436ca48d4850f58eb874. Frozen charter/DONE/slice-8.md SHA-256 is ec75b71c8b17f3dc92df8d79bd673cc47b64dc85187310cc92e251fc5a1f8297, exactly matching the dispatch record; the proof list did not grow.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "HOST_GREEN_VM_PENDING. Host gate command: PATH=\"$HOME/.cargo/bin:$PATH\" CARGO_TARGET_DIR=/tmp/holden-mcp-qa8b-target-host HOLDEN_TARGET_BASE=/tmp/holden-mcp-qa8b-target-host-steps cargo xtask gate slice-8. On macOS/aarch64 at the frozen clean tip: fmt, clippy, tests and guard passed; 143 passed, 0 failed, 0 ignored; all 143 names [ok]. This equals attempt 1's 143-test result and the 135-test slice-7 baseline plus the 8 workflow tests. Run record 20261106T111359Z-macos.json; result green.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "charter check --strict exited 0 with 'charter: ok' and only the pre-existing missing-cost-row warnings for accepted slices 0 through 6. Standalone CARGO_TARGET_DIR=/tmp/holden-mcp-qa8b-target-guard cargo xtask guard exited 0: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all passed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P1 host reopen passed in the full gate: holden-mcp::tests::instructions_name_workflows and workflow_tools_declared_with_schemas. Initialize names W1-W4 in order, all four workflow tools, low-level composition and when to stop and ask the owner; workflow tools have object schemas.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P2 host reopen passed: workflow_tools_compose_low_level_calls and low_level_tools_remain_for_audit. Source and the real run confirm W2 calls actual Core begin_run, uses a real evaluator McpClient for reveal_params/observations/verdict, verifies audit_reveal, writes the signed record and arms the next nonce. The record contains the real reveal proof and failing verdict; low-level tools remain listed.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P3 host reopen passed: prompts_list_all_four_workflows_with_arguments. The real rmcp client listed exactly W1-ONBOARD, W2-CHECK, W3-EXPLAIN and W4-STATUS, retrieved each prompt, and checked required host plus W3 run_id arguments.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P4 host reopen passed: implementer_owner_step_returns_owner_action_required. The implementer-role client received OWNER_ACTION_REQUIRED and the exact 'holden approve --host <canonical-host>' command; no implementer-executable approve tool exists.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P5 positive host reopen passed: e2e_workflows_over_socket and e2e_workflows_over_stdio. Real rmcp clients drove W1 before and after owner approval, W2 on a real broken fixture commit, W3 with a non-leaking fixed explanation, W4 telemetry/authority and low-level audit_reveal over both Unix socket and the stdio bridge.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P1 fail-without mutant in a separate scratch clone removed W2 CHECK from INSTRUCTIONS: tests::instructions_name_workflows selected one test and went red with 'instructions omit W2 CHECK'. Reverted clean; the same fully-qualified named test passed 1/1 in a fresh target directory.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "P2 fail-without mutant in a separate scratch clone made holden_check return brief without composition: tests::workflow_tools_compose_low_level_calls selected one test and went red because evaluated_commit was null. Reverted clean; the named test passed 1/1 in a fresh target directory.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "P3 fail-without mutant in a separate scratch clone dropped W3-EXPLAIN from the prompts list: tests::prompts_list_all_four_workflows_with_arguments selected one test and went red. Reverted clean; the named test passed 1/1 in a fresh target directory.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "P4 fail-without mutant in a separate scratch clone let W1 take the approved path with pending approvals: tests::implementer_owner_step_returns_owner_action_required selected one test and went red with OWNER_APPROVED instead of OWNER_ACTION_REQUIRED. Reverted clean; the named test passed 1/1 in a fresh target directory.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "P5 fail-without mutant in a separate scratch clone skipped begin_run: tests::e2e_workflows_over_socket selected one test and went red with typed could_not_run after reveal_params was refused unknown_run. Reverted clean; the named test passed 1/1 in a fresh target directory.",
      "classification": "executed"
    },
    {
      "id": "C14",
      "description": "D1 CLOSED. Attempt 1's stalled-bridge mutant died only at the verifier's external 90-second timeout (exit 124 after the test had run over 60 seconds). Re-applied at 18c7265, tests::e2e_workflows_over_stdio instead failed itself at the 30-second deadline with exit 101 and a panic naming transport=stdio step=initialize; the external timeout did not fire and no bridge/agent process remained. The mutation was then reverted clean and the unmodified stdio test passed 1/1.",
      "classification": "executed"
    },
    {
      "id": "C15",
      "description": "Repair scope audit passed. Commit 18c7265 changes only crates/holden-mcp/tests/workflows.rs (125 insertions, 40 deletions). All 8 test names are unchanged and the diff changes no assertion. The shared 30-second timeout wraps connect, initialize, call_tool, listings, get_prompt and cancellation with transport/step errors; kill_on_drop(true) kills the stdio child when its transport is dropped on cancellation. No new blocking wait was added inside an exchange deadline. Existing Fixture::stop_agent kill/wait remains unchanged.",
      "classification": "executed"
    },
    {
      "id": "C16",
      "description": "Linux VM gate executed green at the frozen tip (the merged acceptance tip 8ae57b2): 143 passed, 0 failed, 0 ignored; result green_with_declared_limitations — the declared limitations (cargo_deny_absent, cargo_audit_absent, charter_absent in the VM) are covered by the host guard, the standing declared limitation; the host gate at the same merged tip ran green 143/143, and no test-name comparison against the host record was performed at gate time; VM run record 20261107T235615Z-linux-vm.json (hash-recorded in the reopen attestation).",
      "classification": "executed"
    },
    {
      "id": "C17",
      "description": "Acceptance integration: slice-7 (43abdb9, independently QA-accepted: embed/confinement repairs — connect.rs userspace streaming, deadline-thread reaper) merged forward into slice-8 at 8ae57b2 (2 files: holden-embed src/connect.rs + tests/embed.rs only). Host gate at the merged tip: green, 143/143 (20261107T235628Z-macos.json). Linux VM gate at the merged tip: green_with_declared_limitations, 143/143 (20261107T235615Z-linux-vm.json). The prior QA-2 round (18c7265) plus the slice-7 QA-2 round (43abdb9) together cover the merged bytes.",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D1",
      "severity": "medium",
      "title": "Workflow e2e tests can hang indefinitely on a stuck exchange",
      "status": "closed",
      "evidence": "Closed at 18c7265: the same stalled-bridge mutation now yields exit 101 from the test harness within the 30-second initialize deadline, naming transport=stdio step=initialize, rather than attempt 1's external exit 124. No residual bridge/agent process was observed. Evidence: attempt2-teardown-stall-red3.log, attempt2-teardown-revert-green.log and attempt2-repair.diff."
    }
  ],
  "new_defects": [],
  "proofs_reopened": [
    "P1: both instructions/schema tests green on macOS; W2-removal mutant red then green after revert; host: executed; Linux: executed (VM gate green at the merged tip)",
    "P2: composition and low-level audit tests green on macOS; brief-instead-of-composition mutant red then green after revert; host: executed; Linux: executed (VM gate green at the merged tip)",
    "P3: four prompts with arguments green on macOS; W3-removal mutant red then green after revert; host: executed; Linux: executed (VM gate green at the merged tip)",
    "P4: implementer owner boundary green on macOS; approval-path mutant red then green after revert; host: executed; Linux: executed (VM gate green at the merged tip)",
    "P5: real socket and stdio workflows green on macOS; skipped-begin_run mutant red then green after revert; D1 stalled-bridge proof now fails typed at the repaired deadline with no orphan processes; host: executed; Linux: executed (VM gate green at the merged tip)"
  ]
}
