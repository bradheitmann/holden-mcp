# Done list — slice-12b (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-12b --dispatch` (main 24a42fb138a796afe5c26308a71dce215cca98fe).
Scope: Linux owner parity — G-PORTABLE repair node ordered by A/EXEC-PM on 2026-11-08. Branch `slice-12b`, cut from main 24a42fb. The VM gate proved the two owner tests are macOS-only; on Linux an agent running as the owner could still sign from a plain key file. This slice closes that on Linux WITHOUT touching macOS behavior.
Resource scope (exact files): `crates/holden-seal/**` (the Linux backend and its tests), `crates/holden-embed/src/approve.rs`, `crates/holden-embed/tests/embed.rs`, `crates/holden-embed/tests/real_repo.rs` (amendment 1: migrate the Linux approval step at `RealHost::approve` to the established real-PTY pattern — with fail-closed unsealing, the old no-PTY Linux invocation is refused and the Linux full-suite gate would go red), `crates/holden-mcp/tests/workflows.rs` (amendment 3: the VM gate proved the four-workflow e2e tests' Linux approval helper panics at line 271 with the product's own refusal `approve: refused: owner presence required` — migrate it to the real-PTY + passphrase pattern like real_repo.rs; macOS path unchanged; run record 20261108T100154Z-linux-vm.json, result red 171/176 with exactly these five failures), `README.md`, `Cargo.toml` and `Cargo.lock` (dependency additions for the required construction only), `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-12b"`, bump the array length), `crates/holden-cli/tests/**` (amendment 2: the P4 README test's home — tests only, no product src changes; derived from the P4 proof requirement, which needs an existing fail-without test).
Everything else is out of scope. macOS Keychain semantics must not change.

Required construction (EXEC-PM ruling, fixed): the Linux backend seals the owner signing key AT REST with an owner passphrase using Argon2id (key derivation) + XChaCha20-Poly1305 (authenticated encryption). The passphrase is read ONLY from an interactive TTY — never stdin, never an environment variable, never a file — so no non-interactive caller can unseal. Secret Service MAY be used when present but MUST NOT be required. Agents never touch the real owner key; every test uses a disposable sealed keystore with a throwaway passphrase.

Proofs:
1. P1 identical-name parity: `holden-embed::tests::owner_commands_require_presence_and_bars_stay_refused` and `holden-embed::tests::owner_key_resolves_through_keystore_not_file` lose their macOS-only gating and run and pass on macOS AND Linux under the same fully qualified names, each platform exercising its real backend (macOS: disposable Keychain; Linux: disposable passphrase-sealed keystore, approval through a real PTY as in `real_repo.rs`).
   - Reopen: fresh-clone re-run on macOS; Linux half via the VM gate record.
   - Fail-without mutant: point the Linux keystore selection at the plain file backend — the custody test must fail on Linux (signing via the plain file instead of the keystore).
   - Test: the two names above, identical on both platforms.
2. P2 Linux non-TTY refusal: with a sealed keystore present, an approval attempt whose passphrase source is not an interactive TTY is refused with a typed error and produces no signature. The P2 tests are Linux-gated (`cfg(target_os = "linux")`) — a declared, typed platform asymmetry, the mirror of the macOS-only Keychain tests.
   - Reopen: fresh-clone re-run on macOS (compiles, skipped) and via the VM gate on Linux (runs green).
   - Fail-without mutant: make the unseal path accept the key unsealed (skip the interactive-TTY requirement and read the sealed blob without the passphrase) — the test must fail.
   - Test: `holden-embed::tests::non_tty_caller_cannot_approve` and `holden-seal::tests::unseal_requires_interactive_tty`.
3. P3 pre-migration approvals still verify: approvals signed by the owner key BEFORE migration (the plain-file era) still verify against `owner.pub` after the backend change. Verification uses only the public key, so the test runs on BOTH platforms.
   - Reopen: fresh-clone re-run on macOS; Linux half via the VM gate record.
   - Fail-without mutant: make verification reject the pre-migration signature format — the test must fail.
   - Test: `holden-seal::tests::pre_migration_approvals_still_verify`.
4. P4 README owner migration: `README.md` documents the ONE owner command that migrates the real key (`holden migrate-owner-key`) and states that agents never run it. No test or agent executes the migration against the real key.
   - Reopen: fresh-clone re-run on macOS.
   - Fail-without mutant: remove the migration section — `holden-cli::tests::readme_names_the_owner_migration_command` must fail.
Gates (exact commands):
- `cargo xtask gate slice-12b`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-12b --linux-vm` (run by the orchestrator, outer alarm 2700, after the merge-main + pump-grep ritual)
Out of scope (recorded obligations, not fixed here): everything not listed above; macOS Keychain semantics; relay or workflow changes; GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the construction is fixed by the EXEC-PM ruling.
