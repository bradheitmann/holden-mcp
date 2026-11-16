# slice-0 independent QA — attempt 2

**Verdict: ACCEPTED.** This is attempt 2 and references the attempt-1 receipt/report preserved in private agent-state. Attempt 1 rejected only because D1 (Linux reopen) and D2 (live hosted pin call) were unavailable; both are now reopened. All frozen bytes matched before execution.

## Proofs

- **P1 — reopened.** A fresh clean clone passed `cargo xtask gate slice-0` on macOS aarch64: 30/30 tests, format, Clippy, guard, cargo-deny, cargo-audit and charter safety passed. `cargo xtask gate slice-0 --linux-vm` passed on Linux aarch64 with 30/30 tests and identical test names/counts. The VM gate reported declared host-covered limitations because cargo-deny, cargo-audit and charter are absent in the VM. The unconditional macOS-only symbol mutant made the Linux gate red with E0433.
- **P2 — reopened.** The clean-clone fixture verification reported 3 files and digest `981fe61b432a11eb510b406323330bc7b1f0d9218570427b204216f907b8b463`. Appending one byte to `doctrine.md` made the CLI exit 1 and name `differs: doctrine.md`. The absent-root limitation test passed.
- **P3 — reopened.** I initialized the live canon MCP endpoint and called `check_canon_lock` with no arguments. All four hashes, version and validator hash match both committed pin files exactly. The pin comparison test passed; changing one validator-hash digit made it fail with `validator_hash differs`.
- **P4 — reopened.** The typed lens corpus tests passed and the private corpus verified 4 files with digest `3613ee8a88cb6131aaf094806e48e1a7ad2b5801c7c20f39808386e34a0514af`. A timestamp-derived concept id made `lens_ids_are_content_hashes` fail.
- **P5 — reopened.** Required seal tests passed on macOS and Linux. Both fixed-vector tests asserted the same public key, VRF output (`2a32d485332709a85a6b21ba8b983687ba5f63ecbd0c91aefb76a949f46a4a41`), parameter bytes (`3a3a66837b79673ec8edcf4195eaa888cba35bcdb609ca584274ac5eb90f0bef`) and scalar fields. Removing `evaluated_commit` from the transcript made `params_depend_on_evaluated_commit` fail.
- **P6 — reopened with constrained substitute.** Host guard and charter safety passed, along with cargo-deny and cargo-audit. I did not create the literal `tools/x.py` mutant because both the owner hook and task prohibit `.py` files. Removing `py` from the extension matcher made the guard’s language test fail at its expected finding count. Inspection confirmed tracked-file enumeration feeds that matcher; I judge this substitute sufficient to show the Python-extension regression is caught without creating a forbidden file. I created no Dockerfile.

## Additional mutant

Removing extra-file refusal from `holden-canon::verify` made `verify_refuses_extra_file` fail because verification returned `Ok`.

## Defects and obligations

No critical, high, medium or low code defects were found. Attempt-1 D1 and D2 are closed by the Linux VM run and live MCP call. One previously recorded owner decision remains open: `charter/safety-allow.txt` allows `AGENTS.md` because the public COMMON-STANDARD URL contains the owner’s public account handle; the allowance itself says an orchestrator decision is pending. The host safety check passes, and this does not violate P6’s narrower ban on home paths or private source names.

`charter check --strict` returned `charter: ok`. The receipt’s all-proofs reopen attestation is recorded outside the frozen tracked tree and bound to `evidence-attempt2.md`.

## Target-directory execution note

The required `--linux-vm` wrapper does not forward `HOLDEN_TARGET_BASE` to its remote shell, so its first internal remote gate used built-in `/tmp/holden-mcp-target-*` paths. I reran both the frozen Linux gate and P1 mutant gate directly inside their clean VM clones with `CARGO_TARGET_DIR` and `HOLDEN_TARGET_BASE` under the requested `/tmp/holden-mcp-qa-target-*` prefix; those reruns passed and failed as expected. The direct reruns supply the reported Linux evidence.
