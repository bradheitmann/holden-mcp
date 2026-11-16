# Done list — slice-17 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-17 --dispatch` (main c6af8135bd783c295181b3d083f826b40be6021d, plan rev 8).
Scope: host files carry no machine path — the SLICE-17-DEV repair node (plans/2026-11-06-integration/dispatch-graph.json), ordered by A/EXEC-PM: other repositories' PMs refuse host files carrying machine paths, and holden is about to be embedded in those repositories. This slice lands FIRST (18 and 19 depend on it). Branch `slice-17`, cut from main c6af813.
Resource scope (from the graph contract, verbatim): `crates/holden-embed/**` (host, embed, distribution, state), `crates/holden-cli/src/**` (mcp, connect, embed commands), tests, PLUS the two riders A/EXEC-PM authorized (README install covers holden-mcp; migrate-owner-key removes the verified legacy key — both land in holden-cli + README + their tests), PLUS `xtask/src/gate.rs` (the `SLICES` array only: add `"slice-17"`, bump the array length), PLUS AMENDMENT 3 (derived grant): `crates/holden-mcp/src/workflow.rs` (+33 lines — the workflow layer canonicalizes the private user-state alias to the real state directory so the relay resolves the implementer socket at run time; the ruling's runtime-resolution requirement cannot exist without it; same derivation as slice-14's server.rs grant).
Behavior (fixed by the ruling): NOTHING `holden embed` or `holden approve` writes into a host repository contains an absolute path, a home path, a user name or any other machine-specific value. `.mcp.json` no longer carries `--socket <absolute path>`; `.holden/config.json` no longer records the implementer socket path. The relay resolves the socket at run time from the user state directory.
NEW OWNER RULE (2026-11-14, binding, from the real incident): tests NEVER touch the owner's real login keychain — a slice-12 test wrote an item (service holden-mcp.owner-signing, account owner) into it and blocked the owner's migration. Every keychain-touching test uses a disposable named keychain only, and any new keychain rule is tested by ASSERTING ON WRITTEN CONFIG BYTES (the disposable store's name/path in the config) — never by exercising the real login keychain, not even under a mutant. Mutants must never write to the login keychain.
Proofs (the graph's acceptance criteria, each with its fail-without mutant):
1. P1 no machine paths in host files: after `holden embed` on a fixture host, no file written into the host tree contains an absolute path, a home path or the user name — on macOS AND on Linux.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record (identical names and counts); the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: restore the `--socket <absolute>` argument in the written .mcp.json — the test must go red.
   - Test: `holden-embed::tests::host_files_carry_no_machine_paths`
2. P2 cross-machine byte identity: the written .mcp.json is byte-identical across two roots simulating two machines/users — no machine values anywhere in the file.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: embed a path (any machine value) into the file — the cross-root comparison must go red.
   - Test: `holden-embed::tests::mcp_json_byte_identical_across_roots`
3. P3 the harness entry still connects: a real rmcp stdio client spawned from the written .mcp.json completes W1 through W4 against the fixture host — on macOS and Linux.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: none beyond P1's/P2's (this is the no-behaviour-change property; a broken connection fails these existing flows).
   - Test: `holden-cli::tests::written_mcp_json_drives_w1_through_w4`
4. P4 migration + idempotence: re-embed over an old-style .mcp.json (with `--socket <absolute>`) migrates it to the path-free form idempotently, and existing approvals stay valid.
   - Reopen: fresh-clone re-run on macOS; the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: make the migration non-idempotent (re-embed appends or restores the absolute socket) — the test must go red.
   - Test: `holden-embed::tests::reembed_migrates_old_mcp_json_and_keeps_approvals`
5. P5 RIDER — README install covers holden-mcp: the README's install step also installs `crates/holden-mcp` (the holden-mcp binary the relay spawns); a fresh clone following the install has both binaries.
   - Reopen: fresh-clone re-run on macOS (inside the existing quickstart test flow); the Linux half via the VM gate record; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: remove holden-mcp from the README install line — the test must go red.
   - Test: `holden-cli::tests::readme_install_covers_holden_mcp`
6. P6 RIDER — migrate removes the verified legacy key: `holden migrate-owner-key` removes the legacy owner.key file only AFTER verifying the Keychain copy signs identically (verify-then-remove); tested with a DISPOSABLE legacy file in a temp private root and a disposable keychain — the real login keychain and the real owner key are never touched, and a failed verification removes nothing.
   - Reopen: fresh-clone re-run on macOS; the QA re-runs the proof independently before acceptance and its run is reopened and compared.
   - Fail-without mutant: remove-without-verify (delete the legacy file before the signature check) — the test must go red.
   - Test: `holden-cli::tests::migrate_removes_legacy_key_after_verify`
7. P7 AMENDMENT 2 (A/EXEC-PM correction, 2026-11-10) — tests clean their own temp dirs: every new fixture helper's temp dir is removed by a Drop guard, so NO temp dir survives a FAILED test (the 2026-11-10 leaks: red tests panicked before cleanup and left hundreds of /tmp/holden-501-* dirs).
   - Reopen: fresh-clone re-run on macOS; the QA re-runs the proof independently before acceptance and its run is reopened and compared; after the full proof run the orchestrator verifies zero leaked holden-501-* dirs.
   - Fail-without mutant: a fixture helper without the Drop guard, forced to fail — its temp dir leaks and the leak-check must go red.
   - Test: `holden-embed::tests::fixture_temp_dirs_are_drop_guard_cleaned`
DISK + DELETION RULES (A/EXEC-PM, 2026-11-10, binding): ONE CARGO_TARGET_DIR for ALL slice-17 proof runs (/tmp/holden-s17-target, reused); mutants get ONE scratch target (/tmp/holden-s17-mutant) deleted the moment its proof ran; NO other new /tmp/holden-* folder; and EVERY rm names a literal path or "${var:?}" — NEVER a glob (a glob rm deleted the live holden-mcp server's socket dir on 2026-11-10).
Gates (exact commands):
- `cargo xtask gate slice-17`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-17 --linux-vm` (run by the orchestrator, outer alarm 2700, after the merge-main + pump-grep ritual; the VM is currently STOPPED — A/EXEC-PM boots it on request)
Out of scope (recorded obligations, not fixed here): everything not listed above; declared crate paths (slice-18); planted-fault controls (slice-19); the holdout clients (never guessed or imitated); GUARANTEES.yaml and STATE.md edits (acceptance-time acts).
Owner decision points: none — the behavior is fixed by the ruling.
