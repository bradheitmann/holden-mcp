# Done list — slice-12 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-12 --dispatch`
Scope: convergence — obligations, docs quickstart, release candidate. Branch `slice-12`, cut from slice-11 at 1c89304, under the overnight UNPARK ruling (HOST-VERIFIABLE PARTS ONLY tonight). Resource scope: `crates/holden-eval` tests, `xtask`, `README.md`, `docs/`, PLUS the reap-thread bounded wait in `crates/holden-cli` (granted by the overnight ruling's ledger line), PLUS the dispatch-infrastructure registration of "slice-12" in `xtask/src/gate.rs::SLICES` (that array only). Explicitly OUT TONIGHT: GUARANTEES.yaml proven-marking and STATE.md rewrites (they wait for VM proofs + holdout verdicts) — GUARANTEES.yaml and STATE.md must NOT be edited in this run.
Proofs:
1. P1 obligations — each closed by a test with a fail-without mutant:
   a. Cert sentinel (slice-4 D2): the build-child environment leak watch must be tightened so cargo-derived SSL_CERT_DIR/SSL_CERT_FILE values are exempted by an explicit sentinel value, and any OTHER cert-variable leak into a privileged or product process is caught. Expected to land via the watch's inputs and tests; if a product-src change in crates/holden-eval is required, STOP that proof and record the exact files needed (typed blocker; do not edit out of scope).
   - Reopen: fresh-clone re-run on macOS; Linux half when the VM returns.
   - Fail-without mutant: remove the sentinel exemption so the watch flags the cargo-derived values. The watch test must fail.
   - Test: `holden-eval::tests::cert_sentinel_exempts_cargo_and_catches_others`
   b. xtask `--linux-vm` forwards `AGENT_PRIVATE_ROOT` to the VM (a private-root-dependent gate step inside the VM currently fails or degrades without it). In scope (xtask).
   - Reopen: host-side verification of the forwarding logic plus the live VM run when the VM returns; tonight the forwarding is proven by the driver's own test/log.
   - Fail-without mutant: drop the env forwarding. The forwarding test must fail.
   - Test: `xtask::tests::vm_gate_forwards_private_root`
   c. Slice-10 reap-thread bounded wait: the relay's detached reap thread's `wait()` gets a deadline (bounded reap; the runtime thread still never blocks). Granted by the overnight ruling.
   - Reopen: fresh-clone re-run on macOS; Linux half when the VM returns.
   - Fail-without mutant: remove the deadline from the reap wait. The bounded-reap test must fail.
   - Test: `holden-cli::tests::relay_reap_wait_is_bounded`
   d. VM guard tools installed or formally waived: VM-dependent — recorded PENDING tonight, resolved at the VM phase (install in the VM or an owner waiver).
2. P2 quickstart-macos: a fresh clone follows the README quickstart (install, init on a TEST private root, embed, a green /holden check on the fixture host) on macOS. Linux half when the VM returns.
   - Reopen: fresh-clone re-run; the quickstart test performs the steps as a user would.
   - Fail-without mutant: omit the init step from the README. The quickstart test must fail.
   - Test: `holden-cli::tests::readme_quickstart_leads_to_green_check`
3. P3 charter check --strict and the guard pass at the tip (gates). The GUARANTEES.yaml proven-marking component of P3 is DEFERRED by the overnight ruling (requires VM proofs + HOLDOUT-HC1..3) and is recorded here as pending, not skipped.
   - Reopen: fresh-clone re-run of `charter check --strict` and `cargo xtask guard` at the slice tip on macOS; the Linux half when the VM returns.
   - Fail-without mutant: remove a proof's reopen/mutant lines from this DONE list, or introduce a guard-relevant violation (a tracked file carrying a denylisted term) — `charter check --strict` (resp. the guard) must go red. Demonstrated live at dispatch: this proof's initial form lacked the reopen/mutant lines and `charter check --strict` went red naming it.
   - Test: `cargo xtask gate slice-12` (the gate's charter-check and guard steps are the executable form)
4. P4 owner-key custody (macOS Keychain half): the owner signing key is resolved through the OS key store rather than the plain private-root file; existing approvals still verify against the published owner.pub. Fail-without mutant: read the key from the old file path, and the custody test fails. SCOPE REALITY: if the custody path requires product-src changes outside this slice's granted files (holden-seal / holden-cognition), STOP that proof and record the exact files needed (typed blocker; partial delivery is honest).
   - Reopen: fresh-clone re-run on macOS; Linux (Secret Service or passphrase-sealed file) when the VM returns.
   - Test: `holden-seal::tests::owner_key_resolves_through_keystore_not_file` (crate/path follows where the custody test can live within scope; if the test must live elsewhere to be real, record the correct location and stop)
Gates (exact commands):
- `cargo xtask gate slice-12`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-12 --linux-vm` (deferred while VM_OFF; required before any acceptance)
Out of scope (recorded obligations, not fixed here):
- B1/B2 (slice-11's blocked product defects): separate morning decision; not this slice.
- GUARANTEES.yaml proven-marking; STATE.md rewrite (acceptance-time acts).
- Product-src changes in holden-seal/holden-cognition/holden-eval src without a scope grant.
Owner decision points:
- P1d (VM guard tools) and the P4 custody scope reality are resolved at/after the VM phase.
