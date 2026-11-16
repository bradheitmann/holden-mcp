# Done list — slice-11 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-11 --dispatch`
Scope: real-repo end to end, agent-driven — a copy of a real owner repository (made under the private root; the original is never modified) is embedded, and an agent driven only by the MCP instructions and /holden runs W1-W4 across at least two real commits, on macOS and Linux. Branch `slice-11`, cut from slice-10 at 2a17249 with slice-9 (a513c8b) merged in (d560fd8), under the overnight UNPARK ruling (HC-2 has ACCEPTED-status in the graph's normal order; this speculative run builds ahead with the owner's accepted rework risk; VM gates deferred while VM_OFF). Resource scope: `crates/holden-embed/tests/**`, `xtask/src/**` (e2e target only), plus the dispatch-infrastructure registration of "slice-11" in `xtask/src/gate.rs::SLICES` (that array only). The REAL-REPO SEED is a git-clone copy of an owner planning repository, living under the private root at `e2e/real-repo`; the test reads its location ONLY from the environment variable `HOLDEN_E2E_REAL_REPO` (never a tracked path), clones the seed fresh per run into a temp working copy, and the seed itself is never modified. The test fails TYPED (`could_not_run`) if the variable is unset — never a silent skip. No owner project name or path may appear in any tracked file.
Proofs:
1. P1 w1-real-promises-verbatim: W1 on the real repo copy yields promises quoted verbatim from its real planning artifacts (byte-exact spans, per the slice-9 adapter contract).
   - Reopen: fresh-clone re-run at the slice tip on macOS, and in the Linux VM gate with identical test names when the VM returns.
   - Fail-without mutant: paraphrase one extracted promise before approval. The verbatim test must fail.
   - Test: `holden-embed::tests::w1_real_promises_verbatim`
2. P2 two-commits-different-params: two real commits in the copy produce different moment-of-test parameters, and `audit_reveal` verifies both runs.
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: reuse the first commit's parameters for the second. The params test must fail.
   - Test: `holden-embed::tests::two_commits_different_params_audit_verified`
3. P3 planted-regression-named: a planted regression in the copy fails W2, and W3 names the consequence WITHOUT the step, value or threshold.
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: include the threshold in W3's diagnostic. The non-leak test must fail.
   - Test: `holden-embed::tests::planted_regression_fails_w2_w3_names_consequence_only`
4. P4 telemetry-samples: telemetry after the runs reports catch rates WITH sample sizes.
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: report rates without sample sizes. The telemetry test must fail.
   - Test: `holden-embed::tests::telemetry_reports_catch_rates_with_samples`
Gates (exact commands):
- `HOLDEN_E2E_REAL_REPO=<private-root>/e2e/real-repo cargo xtask gate slice-11`
- `HOLDEN_VM_ADDR=192.168.64.10 HOLDEN_E2E_REAL_REPO=<private-root>/e2e/real-repo cargo xtask gate slice-11 --linux-vm` (deferred while VM_OFF; required before any acceptance)
Out of scope (recorded obligations, not fixed here):
- Product changes outside `crates/holden-embed/tests/**` and the xtask e2e target: if the e2e run exposes a PRODUCT defect, record it as a finding and STOP — do not fix it in this slice.
- The owner's original repository: read-only forever; the seed copy under the private root is the only working surface, and per-run commits land in fresh temp clones, never in the seed.
- xtask changes beyond the SLICES registration entry and the e2e target.
Owner decision points:
- None.
