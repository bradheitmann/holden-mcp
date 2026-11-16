{
  "schema_version": "charter.receipt/1",
  "slice": "slice-11",
  "verifies": "slice-11 QA attempt 2 (reissue on the canonical charter build): frozen real-repository P1-P4, adversarial controls, host and Linux gate records",
  "candidate_commit": "cae8b7b12f0ed61b58761f25bf5a37b26101c89a",
  "frozen": {
    "manifest_sha256": "5983ca77a66616461d74cd3e9b5e96cc2003d6b50438548bc4363f27a0b7b7c4",
    "head": "cae8b7b12f0ed61b58761f25bf5a37b26101c89a"
  },
  "dispatch": {
    "base_commit": "40a973ca25b70589c3d8d2b240fb8d4addf1ad4a",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh detached clone is clean at cae8b7b12f0ed61b58761f25bf5a37b26101c89a. charter.freeze/2 independently reopened from the dev worktree names that head/commit, tree 51f48be20536e53c1bf9db5bd68ed9b402761c99, manifest 5983ca77a66616461d74cd3e9b5e96cc2003d6b50438548bc4363f27a0b7b7c4 and 328 entries; all 328 Git object ids and SHA-256 content digests match the clone. DONE sha256 and dispatch done_sha256 both remain ce4cd7c4552429f216ae086dce1250a48a0c0983584e33af2f397cb6c07dafc1. The graph node's P1-P4 acceptance items are unchanged in the DONE list.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "P1-P4 REAL E2E PASS. In the clean detached frozen clone, HOLDEN_E2E_REAL_REPO=<private seed> CARGO_TARGET_DIR=/tmp/holden-mcp-qa11-target-e2e-frozen timeout -k 10 900 cargo test -p holden-embed --test real_repo -- --test-threads=1 exited 0: 4 passed, 0 failed, 0 ignored in 240.48s. The real workflow cloned the private seed with --no-local, embedded and served it, drove W1-W4 through installed MCP instructions/prompts and the stdio relay, evaluated two distinct real commits plus a planted regression, independently verified closed audit proofs, recorded real controls and bounded a stalled child. w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples all passed.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 FAIL-WITHOUT RED/GREEN. In the isolated mutant clone, charter promise extraction changed the first owner's text to a paraphrase before approval. w1_real_promises_verbatim went RED (exit 101, 13.94s), refusing the quote as not equal to the source bytes at its recorded span. Exact revert was clean and the same fully qualified test went GREEN 1/1 in 241.40s. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 FAIL-WITHOUT RED/GREEN. After the real second-run audit had independently verified, the QA fault-injection mutant replaced the observed second commit params with commit one's params for the matching plan. two_commits_different_params_audit_verified went RED at its reused-params assertion (exit 101, 234.50s). Exact revert was clean and the same test went GREEN 1/1 in 212.75s. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "P3 FAIL-WITHOUT RED/GREEN. Adding a threshold field to W3's fixes made planted_regression_fails_w2_w3_names_consequence_only go RED with W3 leaked threshold (exit 101, 221.76s). Exact revert was clean and the same test went GREEN 1/1 in 214.08s. Source review confirms W3 emits only promise and consequence in fixes. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P4 FAIL-WITHOUT RED/GREEN. Removing n from rate_json made telemetry_reports_catch_rates_with_samples go RED with no real catch trials (exit 101, 207.13s). Exact revert was clean and the same test went GREEN 1/1 in 198.10s. The proof requires nonzero real catch samples and independently recomputes point and Wilson lower/upper values from k and n. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "W2 BOUNDARY + FIELD MUTANT PASS. A fresh focused run of w2_response_carries_no_param_fields and w2_emits_monotonic_non_leaking_progress passed 2/2 in 24.05s. The W2 source composes using the server's evaluator config; the implementer response has only workflow, status, evaluated_commit, public run diagnostics/authority and aggregate telemetry. Adding params to a run response made w2_response_carries_no_param_fields RED (22.10s); exact revert went GREEN 1/1 (22.39s). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "PROGRESS + EMISSION MUTANT PASS. The focused real-rmcp run verified nonempty strictly monotonic notifications, stable tokens and exactly the fixed public phase strings; no param/seed/nonce/proof/cipher/threshold/step/value/promise/observation text is permitted. Removing peer.notify_progress made w2_emits_monotonic_non_leaking_progress RED with W2 emitted no progress (21.94s); exact revert went GREEN 1/1 (22.17s). Never committed.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "RELAY MUTANTS RED/GREEN. Replacing runtime.shutdown_timeout(REAP_DEADLINE) with plain drop(runtime) made relay_idle_deadline_ends_held_stdin_session RED at the typed held-stdin deadline (52.09s); exact revert went GREEN 1/1 (27.83s). Wrapping the exchange in a 60-second connection-age timeout made relay_session_survives_long_active_session RED with relay deadline connection age and TransportClosed (85.18s); exact revert went GREEN 1/1 after 167.10s, proving active sessions survive beyond the 120-second age threshold. Neither mutant was committed.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "STARTUP REGRESSION RED/GREEN. Removing only the /\\.CFUserTextEncoding$ allowance from macos-product.sb made the real e2e's first W2 return refusal could_not_run, the sealed evaluation could not finish, and the named test went RED in 103.54s. Exact restoration was clean and the same test went GREEN 1/1 in 224.12s. The allowance is a single filename regex, not a home-directory subpath. Never committed.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "B1 ADVERSARIAL INGESTION PASS. embed.rs always calls holden_cognition::adapters::read_promises; --planning is retained as public compatibility metadata and never creates a host-flow bypass. extract_promises remains the legitimate canonical parser, used by the Charter adapter for explicit PROMISE spans, the Markdown adapter, synthetic fixture/unit tests and downstream evaluator/generator fixtures. The only holden_embed::host::planning call is a synthetic embed test reopening its fixture, not production host flow.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "PRODUCT BINDING AUDIT PASS. ProductBinding is a deny_unknown_fields tagged enum (cargo_workspace or explicitly declared fixed boundary). discovery uses cargo metadata --offline --locked --no-deps, selects sorted/deduped real workspace bin targets, and returns typed UnsupportedStack when Cargo.toml or all CLI entry points are absent; metadata failures are typed and there is no notes fallback for an unknown host. Evaluation uses cargo build --offline --locked --workspace --message-format=json and selects artifacts only from compiler messages under the fresh world's target. The e2e asserted cargo_workspace and at least two real CLIs.",
      "classification": "executed"
    },
    {
      "id": "C13",
      "description": "SEED DISCIPLINE + SCOPE PASS. No home-anchored absolute paths, no user-specific paths, and no concrete agent-private seed path appear in tracked files; only the generic e2e/real-repo relative contract/fallback exists. The per-run seed is cloned afresh and the seed is never written. git diff --check 2a17249..cae8b7b exited 0. The 91-file scope diff has zero paths outside the direct grants plus inherited slice-9 fixtures/adapters, slice-10 relay/distribution, slice-12 seal custody/forwarding, accepted main plans/QA/charter records, Cargo.lock/Cargo.toml and xtask. No main or branch file was modified by QA.",
      "classification": "executed"
    },
    {
      "id": "C14",
      "description": "HOST/VM EVIDENCE + EXACT ASYMMETRY PASS. Reopened 20261108T063446Z-macos.json: commit cae8b7b, result green, 170 passed/0 failed/0 ignored, 170 unique names all [ok], fmt/clippy/tests/guard pass and guard_verdict pass. Reopened 20261108T064856Z-linux-vm.json without VM contact: the same commit, tree_clean true, result green_with_declared_limitations, 169 passed/0 failed/0 ignored, 169 unique names all [ok]. comm of the sorted name sets has exactly one host-only line, holden-embed::tests::owner_key_resolves_through_keystore_not_file [ok], and zero VM-only lines. That test is #[cfg(target_os = macos)] and exercises a throwaway Keychain against a decoy owner.key; the Linux Secret Service/passphrase-sealed custody half is declared-deferred by slice-12 P4. This is a typed platform limitation, not a failure. VM-only cargo_deny_absent/cargo_audit_absent/charter_absent are covered by the passing host guard. Host tree_clean false is explained by the untracked freeze file alone; all tracked frozen bytes match independently.",
      "classification": "executed"
    },
    {
      "id": "C15",
      "description": "CHARTER + GUARD PASS. charter check --strict at the isolated tip exited 0 with charter: ok and only the standing accepted-slice cost-row warnings. Standalone cargo xtask guard with its own CARGO_TARGET_DIR exited 0: languages, absolute_home_paths, private_identifiers, stale_license_mentions, license_text, crate_licenses, cargo_deny, cargo_audit and charter_safety all pass; verdict Pass. The mutant clone was clean after every exact revert and no mutant was committed.",
      "classification": "executed"
    },
    {
      "id": "C16",
      "description": "ATTEMPT-1 TOOL MISMATCH RESOLVED. The stale PATH charter cf05eb5 refused the mandated verifier_model minimax-m3.1-flash (declared_by: droid-mission harness) as absent from its family table. That refusal was not a product defect or a real receipt gap. The canonical build b7a513b knows the required model and validates the accepted reissue without a waiver; see C17. No tracked file, proof, model id or acceptance bar was changed.",
      "classification": "executed"
    },
    {
      "id": "C17",
      "description": "Receipt validated with the canonical charter build (commit b7a513b, the project's pinned release build): charter receipt --verify-tree <clone> <receipt> exit 0, verdict accepted",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D-QA-RECEIPT-1",
      "severity": "info",
      "status": "resolved",
      "description": "Tool mismatch: the stale PATH charter cf05eb5 refused the mandated verifier id; the canonical build b7a513b validates it without a waiver. No product defect and no remaining receipt gap."
    }
  ],
  "new_defects": [],
  "proofs_reopened": [
    "P1 host executed independently 1/1; supplied Linux executed [ok]; paraphrase mutant RED then exact revert GREEN",
    "P2 host executed independently 1/1; supplied Linux executed [ok]; reused-params QA mutation RED then exact revert GREEN; closed audits verified before mutation",
    "P3 host executed independently 1/1; supplied Linux executed [ok]; W3 threshold mutant RED then exact revert GREEN",
    "P4 host executed independently 1/1; supplied Linux executed [ok]; missing-sample-size mutant RED then exact revert GREEN"
  ]
}
