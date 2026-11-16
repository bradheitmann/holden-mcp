# Done list — slice-10 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-10 --dispatch`
Scope: distribution and connection — one-time machine setup plus per-repo connection so any MCP client (Claude Code, Codex, Droid) can use holden, and so a user can drive everything through a single / command. Branch `slice-10`, cut from slice-8 at 18c7265 under the overnight SPECULATIVE_DISPATCH ruling (HC-1 still gates acceptance; VM gates deferred while VM_OFF). Resource scope: `crates/holden-cli` (init, mcp relay), `crates/holden-embed` (harness config writers, command and skill files), `packaging/` (no launchd or systemd loading), tests. Additionally authorized as dispatch infrastructure: registering "slice-10" in `xtask/src/gate.rs::SLICES` (that file, that array only — no other xtask change). MUST NOT touch the planning adapters (slice-9's lane: `crates/holden-cognition` adapters / `crates/holden-plan-adapters`).
Proofs:
1. P1 init-safe-idempotent: `holden init` creates the sealed server config and keys in the private root and key store, idempotently; it refuses world-readable paths; no secret is ever printed.
   - Reopen: fresh-clone re-run at the slice tip on macOS, and in the Linux VM gate with identical test names when the VM returns.
   - Fail-without mutant: allow group-readable key files. The path-mode refusal test must fail.
   - Test: `holden-cli::tests::{init_is_idempotent, init_refuses_world_readable_paths, init_never_prints_secrets}`
2. P2 stdio-relay: `holden mcp` is a stdio MCP server process for clients to spawn; it relays to the sealed socket server, starting it on demand as a child process (never launchctl or systemd load), and carries the implementer role; a real rmcp stdio test passes on both OSes.
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: have the relay serve the evaluator role. The role test must fail.
   - Test: `holden-cli::tests::{mcp_stdio_relay_spawns_server_on_demand, mcp_stdio_relay_carries_implementer_role}`
   - HAZARD (the slice-7/8 lesson, binding for this slice): the relay spawns child processes — the relay AND its tests must keep child stdin write ends closing so EOF propagates, every exchange must be bounded so a stuck child FAILS TYPED (deadline thread or tokio timeout on the async future; never a blocking wait on the runtime thread), and children must be killed and reaped on every exit path. A test that can hang is itself a defect.
3. P3 harness-configs: `holden embed` writes connection config for Claude Code (.mcp.json plus a /holden command or skill), Codex (mcp_servers entry) and Droid (mcp config plus a custom command); each file validates against its harness's format, and re-embedding is idempotent.
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: write a Codex entry into the Claude Code config. The format-validation test must fail.
   - Test: `holden-cli::tests::{embed_configs_validate_per_harness, embed_is_idempotent}`
4. P4 slash-command: `/holden` with a sub-argument `onboard`, `check`, `explain` or `status` maps 1:1 to W1-W4 and only calls MCP workflow tools; owner steps print the owner command instead of acting.
   - Reopen: fresh-clone re-run on macOS, plus the Linux VM gate when available.
   - Fail-without mutant: let /holden approve call a low-level approval tool. The mapping test must fail.
   - Test: `holden-cli::tests::holden_command_maps_to_workflows_only`
Gates (exact commands):
- `cargo xtask gate slice-10`
- `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-10 --linux-vm` (deferred while VM_OFF; required before any acceptance)
Out of scope (recorded obligations, not fixed here):
- Planning adapters (slice-9's lane); any holden-cognition/holden-plan-adapters change.
- launchd/systemd loading of any kind; xtask changes beyond the single SLICES registration entry.
Owner decision points:
- None.
