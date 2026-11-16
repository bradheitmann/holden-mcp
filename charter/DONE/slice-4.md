# Done list — slice-4 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-4 --dispatch`
Scope: completion criterion 9 and the never-again controls. `holden-eval` runs a sealed plan against a real product at the boundary on macOS and Linux and records a signed, replayable verdict. The MCP server gains `submit_observation` and `verdict`.
Proofs:
1. P1 real-product-run: a committed fixture product (a small Rust CLI under `fixtures/products/notes`) is built from source in a scratch worktree at an evaluated commit and observed only at its boundary: process exit, stdout/stderr, and files. It has three committed variants, each its own git commit in a fixture repository: `keeps` (correct), `breaks` (silently drops data) and `refuses` (errors). For one approved plan the verdicts are pass, fail and fail. Each fail diagnostic names the promise and the consequence and never the step, value or threshold ("publish the why, seal the how").
   - Reopen: fresh clone; run the integration test on the host and in the VM.
   - Fail-without mutant: make the check compare against the builder-visible card instead of the planted value. The test `breaks_variant_fails` must fail.
   - Test: `holden-eval::tests::{keeps_passes, breaks_variant_fails, refuses_variant_fails, diagnostic_hides_how}`
2. P2 parameters-from-server-only: the evaluator obtains the moment-of-test parameters only through MCP `reveal_params`, under the evaluator role. It plants the values derived from them, such as names and interruption points. A run whose parameters were not revealed by the server for that evaluated commit is refused. The same plan run against two different evaluated commits uses different planted values.
   - Reopen: run the tests.
   - Fail-without mutant: let the evaluator compute parameters locally. The test `params_must_come_from_server` must fail.
   - Test: `holden-eval::tests::{params_must_come_from_server, planted_values_differ_per_commit}`
3. P3 signed-replayable-verdict: each verdict record includes everything needed to reach it again: plan commit, evaluated commit, parameter proof, captured boundary observations (bounded and hashed), rule and engine versions, and the decision. The record is signed with Ed25519 by the evaluator key. `holden replay <record>` recomputes the decision from the stored inputs alone and must match. A changed observation or a changed signature is refused.
   - Reopen: replay every record produced by the run on both OSes.
   - Fail-without mutant: drop the observations from the record. The test `replay_reproduces_decision` must fail.
   - Test: `holden-eval::tests::{verdict_is_signed, replay_reproduces_decision, tampered_record_is_refused}`
4. P4 confinement-no-docker: the product runs under a confinement backend. On Linux this is Landlock, restricting filesystem writes to the world directory and denying network where the kernel supports it, plus rlimits. On macOS it is a `sandbox-exec` profile denying network and writes outside the world directory, plus rlimits. A fixture product variant that tries to write outside its world directory or open a network socket fails to do so, and the attempt is recorded. The implementer's environment variables are never passed to any process holden starts with elevated rights, and the product gets a scrubbed environment.
   - Reopen: run the tests on both OSes.
   - Fail-without mutant: run the product with no confinement. The test `escape_attempt_is_blocked` must fail.
   - Test: `holden-eval::tests::{escape_attempt_is_blocked, product_env_is_scrubbed}`
5. P5 never-again-controls: these defects found in the S-RUNNER review are covered by tests that fail if the defect is reintroduced.
   - (a) The evaluated commit's own tree is scanned for leaked canary tokens, not a path the caller chooses.
   - (b) Any change to the project's public config is diffed and reported in the verdict.
   - (c) Limits on capture bytes and run seconds are enforced, not merely computed.
   - (d) World ids are unique per evaluation, so concurrent runs cannot collide.
   - Reopen: run the tests.
   - Fail-without mutant: scan a caller-supplied path instead of the evaluated tree. The test `canary_scan_uses_evaluated_tree` must fail.
   - Test: `holden-eval::tests::{canary_scan_uses_evaluated_tree, config_change_is_reported, limits_are_enforced, concurrent_runs_do_not_collide}`
6. P6 generated-plans-run: a plan produced by `holden-gen` (slice 3) from the synthetic fixture can be registered with the MCP server and evaluated end to end. The generator-to-server-to-evaluator path is one typed plan format, and the server no longer accepts only the slice-1 plan type.
   - Reopen: generate, register and run a fixture plan on the host and in the VM.
   - Fail-without mutant: make registration accept only the slice-1 plan type. The test `generated_plan_registers_and_runs` must fail.
   - Test: `holden-eval::tests::generated_plan_registers_and_runs`
Base note: this branch builds ahead on the unaccepted slice-1, slice-2 and slice-3 candidates. Acceptance requires those first. Also correct `charter/GUARANTEES.yaml` G-SITUATED proof names to match the DONE lists (`unapproved_cognition_cannot_host_test` lives in `holden-cognition`), and record the slice-3 doctrine decision that the regenerative-loop root cites the canon spec's Section 8, THE LEARNING LOOP.
Gates: same as slice-2.
Out of scope: telemetry and authority (slice-5); other product kinds such as HTTP services (later); separate-OS-user worlds (later).
Owner decision points: none.
