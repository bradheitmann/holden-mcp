# Slice-4 verifier evidence — attempt 2

## Frozen bytes and prerequisites

- Frozen candidate: `047a0000153b857a38d6e589051501c810b1a641`.
- Attempt 1 executed the byte verification: 211 manifest entries, tree `96eb5bfc8c95adb8355c76dbf4c5377c1672e28a`, manifest SHA-256 `fa62c79cf2d59c244eb15a155d9c87866f8235be6194add862fe1d6eee2d5f9d`, per-file SHA-256 and Git blob identities matched. Patch SHA-256: `aa9038654cce40fb4c9120a21372f42a4259805e3b329485297913b3d01cee70`.
- Attempt-1 archive was confirmed present at the private evidence location: `receipt.json`, `report.md`, and `evidence.md`. Its prior executed results are referenced below rather than rerun.
- Cargo: `1.95.0 (f2d3ce0bd 2026-03-21)`.
- Prerequisites were verified in attempt 2 from the main checkout, read only. Each command used a clean scratch clone at the head from its freeze manifest:
  - Slice-1 `cc677f6f4febc4ffaffc378aac2aea51a796be89`: tree matches the recorded freeze.
  - Slice-2 `c4d998e5ab8315c5aec5a3ec82bd71d9d9454796`: tree matches the recorded freeze.
  - Slice-3 `99a5bbfe14c935f470acec8bd6adac78f058bc20`: tree matches the recorded freeze.

## Attempt 2 host gate

- Clean scratch clone: `/tmp/holden-mcp-qa4b-host.I47ywe`, detached at the frozen candidate; `git status --porcelain` was empty before the run.
- Command: `CARGO_TARGET_DIR=<unique /tmp/holden-mcp-qa4b-* directory> HOLDEN_TARGET_BASE=/tmp/holden-mcp-qa4b-target cargo xtask gate slice-4`.
- Result: green on macOS/aarch64. fmt, clippy, tests, s4-replay and guard passed; 111 tests passed, 0 failed, 0 ignored. All 24 verdict records replayed to matching decisions. Host test count matches the Linux VM count from attempt 1.
- Compared the retained attempt-1 Linux VM run record (`20261105T231625Z-linux-vm.json`) with the attempt-2 macOS host run record (`20261105T235918Z-macos.json`): all 111 ordered test names are identical, and both report 111 passed, 0 failed, 0 ignored.
- Host guard passed languages, absolute home paths, private identifiers, stale license mentions, license text, crate licenses, cargo-deny, cargo-audit and charter safety.

## Attempt 2 host M4 confinement mutant

In the clean scratch clone, changed `Confiner::run` to run the product `Spawn` directly, bypassing `self.wrap(...)` and its macOS `sandbox-exec` path.

- Mutant command: `CARGO_TARGET_DIR=/tmp/holden-mcp-qa4b-target-m4-red cargo test --locked -p holden-eval --test eval -- --exact tests::escape_attempt_is_blocked`.
- Mutant result: red, exit 101. The unconfined process ran directly and the test failed because the confinement launcher report was absent.
- Restored `crates/holden-eval/src/confine.rs` from the frozen Git tree; `git diff --exit-code` confirmed the file was restored.
- Revert command: `CARGO_TARGET_DIR=/tmp/holden-mcp-qa4b-target-m4-green cargo test --locked -p holden-eval --test eval -- --exact tests::escape_attempt_is_blocked`.
- Revert result: green, 1 passed.

## Attempt-1 executed results relied upon

The archived attempt-1 receipt and evidence record report the following executed checks:

- Linux VM gate: green, 111 passed and 0 failed; 24/24 records replayed; Landlock `escape_attempt_is_blocked` and environment `product_env_is_scrubbed` passed. VM guard limitations were cargo-deny, cargo-audit and charter unavailable; attempt-2 host guard passed all three.
- Mutants M1–M6 each turned their named test red and turned green after revert: `breaks_variant_fails`, `params_must_come_from_server`, `replay_reproduces_decision`, `escape_attempt_is_blocked`, `canary_scan_uses_evaluated_tree`, and `generated_plan_registers_and_runs`.
- Independent P3 signature mutant turned `tampered_record_is_refused` red and green after revert.
- Fixture review found a committed real Rust CLI built from the evaluated fixture commit and observed only at its process/file boundary; no mocks or stubs were found.

## Repair judgment

The repair exempts cargo-derived `SSL_CERT_DIR` and `SSL_CERT_FILE` from the build-child leak watch. The product's exact environment allow-list and runtime `env_clear` remain. This narrows the regression check for forwarding those two names into build children, but does not weaken the product's exact allow-list. Per the orchestrator's acceptance, D2 is retained as a medium obligation for a later slice and does not block this verdict.

## Checklist

- [x] Frozen bytes verified against the freeze manifest (executed in attempt 1; archived evidence confirmed present).
- [x] Prerequisites verified (three receipts, executed in attempt 2).
- [x] Host gate green: 111/111 (executed in attempt 2).
- [x] Linux gate green, 111/111; all test names and counts match the host (Linux gate executed in attempt 1; run records compared in attempt 2).
- [x] M1–M6: executed in attempt 1; red under mutation, green after revert. Host M4 additionally executed in attempt 2.
- [x] Independent P3 mutant: executed in attempt 1; red under mutation, green after revert.
- [x] Repair judged; D2 recorded as accepted medium obligation.
- [x] Fixture built from source at evaluated commit and observed at boundary; no mocks (executed in attempt 1, now covered by green host gate).
