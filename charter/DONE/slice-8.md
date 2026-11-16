# Done list — slice-8 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-8 --dispatch`
Scope: the self-composing MCP workflow layer. An agent that has never seen holden completes the four standard workflows using only what the MCP server publishes at initialize (instructions, tool descriptions, prompts): W1 ONBOARD (discover the repo's promises and propose cognitions for owner approval), W2 CHECK (run a just-in-time sealed holdout on the current commit and return the fixed diagnostic), W3 EXPLAIN (turn a failed verdict into what to fix, without leaking the how), W4 STATUS (telemetry, authority and pending owner actions). Branch `slice-8`, cut from slice-7 tip c5f1903; rebased onto main once SLICE-07-QA lands.
Proofs:
1. P1 instructions-name-workflows: `initialize` returns non-empty server instructions that name W1-W4, their tool order, and when to stop and ask the owner; a test asserts each workflow's tools exist with schemas.
   - Reopen: fresh-clone re-run of the instructions tests at the slice tip on macOS, and in the Linux VM gate with identical test names.
   - Fail-without mutant: remove W2 (CHECK) from the instructions text. The instructions test must fail.
   - Test: `holden-mcp::tests::instructions_name_workflows` and `holden-mcp::tests::workflow_tools_declared_with_schemas`
2. P2 workflow-tools-compose: one workflow tool per workflow (`holden_onboard`, `holden_check`, `holden_explain`, `holden_status`) composes the lower-level tools server-side, so the client needs one call per workflow; the low-level tools remain for audit.
   - Reopen: fresh-clone re-run on macOS plus the Linux VM gate.
   - Fail-without mutant: make `holden_check` return the brief without running `begin_run`/`reveal_params` (stop composing). The composition test must fail.
   - Test: `holden-mcp::tests::workflow_tools_compose_low_level_calls` and `holden-mcp::tests::low_level_tools_remain_for_audit`
3. P3 prompts-list-workflows: MCP prompts list W1-W4 as invocable prompts with arguments.
   - Reopen: fresh-clone re-run on macOS plus the Linux VM gate.
   - Fail-without mutant: drop W3 (EXPLAIN) from the prompts list. The prompts test must fail.
   - Test: `holden-mcp::tests::prompts_list_all_four_workflows_with_arguments`
4. P4 owner-steps-never-implementer-executable: owner-only steps (approval, bars) are never executable by the implementer role: the workflow returns `OWNER_ACTION_REQUIRED` with the exact owner command; a test proves the implementer role cannot approve.
   - Reopen: fresh-clone re-run on macOS plus the Linux VM gate.
   - Fail-without mutant: let the implementer role's `holden_onboard` reach the approval path instead of returning `OWNER_ACTION_REQUIRED`. The role test must fail.
   - Test: `holden-mcp::tests::implementer_owner_step_returns_owner_action_required`
5. P5 real-client-end-to-end: a real rmcp client test drives W1→W2→W3→W4 end to end against the fixture host, over a socket and over stdio, on macOS and Linux.
   - Reopen: on both OSes (Linux via the VM gate), from a fresh clone, over a socket and over stdio.
   - Fail-without mutant: skip the `begin_run` step in `holden_check`'s composition. The end-to-end test must fail.
   - Test: `holden-mcp::tests::{e2e_workflows_over_socket, e2e_workflows_over_stdio}`
Gates (exact commands):
- `cargo xtask gate slice-8`
- `HOLDEN_VM_ADDR=$(tart ip holden-mcp-linux) cargo xtask gate slice-8 --linux-vm` (test names and pass counts must match the host)
Out of scope (recorded obligations, not fixed here):
- Changing low-level tool semantics beyond server-side composition (they remain for audit).
- New crates; harness config writers and packaging (slice-10); planning adapters (slice-9).
- Any file outside `crates/holden-mcp/**`, `crates/holden-cli/src/**` (workflow subcommands only) and tests.
Owner decision points:
- None. Owner-only steps return `OWNER_ACTION_REQUIRED` with the owner command; the owner node OWNER-APPROVE-DEMO is tracked separately in the dispatch graph.
