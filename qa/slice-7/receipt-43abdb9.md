{
  "schema_version": "charter.receipt/1",
  "slice": "slice-7",
  "verifies": "slice-7 QA attempt 2 + Linux VM gate reopen; cross-platform acceptance",
  "candidate_commit": "43abdb954153c8bfbeabb7fc3ab2eb8988a13beb",
  "frozen": {
    "manifest_sha256": "03d77f3d8a85ed1b7de7649032c11b36298ad5359649ea67a2c4bed9d5867beb",
    "head": "43abdb954153c8bfbeabb7fc3ab2eb8988a13beb"
  },
  "dispatch": {
    "base_commit": "b875d96c0d2d1e3615cc313b13f593697b5280ff",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "Frozen-byte identity: fresh detached scratch clone at 43abdb954153c8bfbeabb7fc3ab2eb8988a13beb, clean status; supplied charter.freeze/2 head, kind, manifest SHA-256 and entry count matched. Recomputed SHA-256 and Git blob identity for all 277/277 entries; every entry matched.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "Host gate on macOS/aarch64 at the frozen tip: PATH=$HOME/.cargo/bin:$PATH CARGO_TARGET_DIR=/tmp/holden-mcp-qa7b-target-gate cargo xtask gate slice-7. Result green; fmt, clippy, tests, s4-replay (37 records), s5-ledger-verify (12 ledgers), selftest (5/5 catches), and guard passed. Test record reopened: 135 expected names, 135 [ok], 0 failed, 0 ignored.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 host reopen: holden-embed installs_expected_files, no_sealed_bytes_in_host and embed_is_idempotent each [ok] in the full gate. The tests construct fresh real fixture-host Git repositories, run the real embed twice where applicable, inspect expected files and modes, forge the constitution, and scan host bytes against real agent/owner keys, sealed ciphertext and plaintext canaries.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 host reopen: holden-embed host_implementer_cannot_reveal [ok]. The test starts a distinct real agent process and the installed stdio bridge, checks the implementer brief's public-only fields and forbidden_role reveal refusal, and confirms the evaluator can reveal the same open run.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 host reopen: post_commit_runs_jit_holdout and consecutive_commits_get_different_params [ok]. Real post-commit hooks evaluated four plans on each real commit; the tests checked ancestor-armed nonces, signed ledger entries, fixed diagnostics, different parameters, and audit equivalence. holden trace and holden replay succeeded for all 12 runs exercised by the two tests (4 plus 8).",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 host reopen: OWNER-APPROVE-DEMO.done records four approvals. Using only $AGENT_PRIVATE_ROOT/holden-mcp/owner-keys/owner.pub, the published key equaled c4f5016859edabe617bf2378007c1b2ae925c02e2fc70531ba954d9edfa38176. Independent OpenSSL Ed25519 verification over holden-cognition/approval/1 || cognition-id || promise-id verified 4/4 approval signatures, with each record naming the same published public key. holden-cognition tests::agent_signed_approval_is_refused also passed as an exact single-test run.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P5 host reopen: holden-eval sealed_read_is_denied and holden-ledger block_reachable_when_bounds_met [ok]. On macOS the real sandbox-exec/SBPL path killed the peeks fixture on its sealed-file read, leaked no sealed bytes, carried no reads_not_confined limitation, and allowed qualifying measured bounds to reach block.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P1 fail-without mutant in a separate fresh clone: prepare_plans also wrote each sealed plan into .holden/. no_sealed_bytes_in_host went RED (exit 101), explicitly finding sealed plan hex in the host. Reverted clean; the same exact test went GREEN.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "P2 fail-without mutant in a separate fresh clone: installed the host MCP entry with evaluator role and evaluator socket. host_implementer_cannot_reveal went RED (exit 101; caller_role evaluator rather than implementer). Reverted clean; the same exact test went GREEN. A preliminary argument-only role edit left the real socket authority unchanged and was discarded, not counted as the mutant proof.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "P3 fail-without mutant in a separate fresh clone: jit reused the preceding run's recorded parameters. consecutive_commits_get_different_params went RED (exit 101) on the parameter mismatch. Reverted clean; the same exact test went GREEN.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "P4 fail-without mutant in a separate fresh clone: admit_approved trusted the key named by each approval instead of the published owner key. agent_signed_approval_is_refused went RED (exit 101; agent-signed approval admitted). Reverted clean; the same exact test went GREEN.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "P5 fail-without mutant in a separate fresh clone: removed the macOS product file-read deny and its parent-read self-check deny. sealed_read_is_denied went RED (exit 101; planted sealed bytes read) and block_reachable_when_bounds_met went RED (exit 101; ReadsNotConfined appeared). Reverted clean; both exact tests went GREEN.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "Repair adversarial audit (a,b): git show --stat and diff-tree prove 43abdb9 changes only crates/holden-embed/src/connect.rs and crates/holden-embed/tests/embed.rs. No io::copy/std::io::copy call remains on the bridge path; both directions use explicit read/write_all/flush userspace streaming.",
      "classification": "executed"
    },
    {
      "id": "C14",
      "description": "Repair adversarial audit (c): the dedicated stdio-child-deadline thread owns Child, waits on the finished channel with a deadline, then kills and wait-reaps on timeout; post-completion polling has the same deadline. A separate fresh-clone adversarial mutant made the bridge never answer and reduced the deadline to 2 seconds. host_implementer_cannot_reveal went RED with ConnectionClosed(initialize response), rather than hanging. The mutant was reverted clean.",
      "classification": "executed"
    },
    {
      "id": "C15",
      "description": "Repair adversarial audit (d): complete repair diff inspected. Existing brief-field, role, forbidden_role, evaluator-reveal and sealed-byte assertions remain; the child success assertion remains after joining the deadline thread. The prior single-runtime outer timeout was replaced by non-runtime child ownership and kill/reap; no guarantee, acceptance bar, or verdict assertion was weakened.",
      "classification": "executed"
    },
    {
      "id": "C16",
      "description": "charter check --strict at the frozen tip exited 0 with charter: ok. It emitted only the pre-existing missing cost-row warnings for accepted slices 0, 1, 1b, 1c, 2, 3, 4, 5 and 6.",
      "classification": "executed"
    },
    {
      "id": "C17",
      "description": "cargo xtask guard at the frozen tip with its own CARGO_TARGET_DIR exited 0: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety passed; verdict Pass.",
      "classification": "executed"
    },
    {
      "id": "C18",
      "description": "Linux VM gate executed green at the frozen tip: 135 passed, 0 failed, 0 ignored; test names identical to host record 20261106T101848Z-macos.json; pass counts identical (host 135 / VM 135); result green_with_declared_limitations — the declared limitations (cargo_deny_absent, cargo_audit_absent, charter_absent in the VM) are covered by the host guard, the standing declared limitation; VM run record 20261107T235353Z-linux-vm.json (hash-recorded in the reopen attestation).",
      "classification": "executed"
    }
  ],
  "defects": [],
  "new_defects": [],
  "proofs_reopened": [
    "P1 host: executed; P1 Linux: executed (VM gate green)",
    "P2 host: executed; P2 Linux: executed (VM gate green)",
    "P3 host: executed; P3 Linux: executed (VM gate green)",
    "P4 owner public-key verification and refusal test: host: executed; Linux: executed (VM gate green)",
    "P5 host: executed; P5 Linux: executed (VM gate green)"
  ]
}
