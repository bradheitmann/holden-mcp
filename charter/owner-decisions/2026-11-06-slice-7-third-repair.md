# Owner decision — 2026-11-06 slice-7-third-repair

approved_by: owner

## Context

Repair 2 for slice-7 (afbe13d) was test-only: it closed the bridge's stdin and added tokio timeouts. The Linux VM gate still hung in `host_implementer_cannot_reveal`.

The evidence came from /proc on the VM:
- The test thread was blocked in a synchronous `read()` on the bridge's stdout (fd 23), so the tokio timeout could not fire.
- The `holden connect` bridge was idle: one thread was waiting on its stdin pipe and the other on the server socket.
- The agent server was idle.

So a request sent through the bridge on the implementer socket gets no answer on Linux, while macOS passes. The defect is in product code, not only in the test.

## Decision

The owner chose "Allow product fix" (2026-11-06): one more repair cycle for slice-7.

Scope:
- `crates/holden-embed/src/connect.rs`
- the agent and MCP server's socket accept and handling path (`crates/holden-embed/src/agent.rs`, `crates/holden-mcp/src/**` socket transport)
- `crates/holden-embed/tests/embed.rs`

Requirements:
- Every stdio exchange in the tests must be bounded in a way that cannot block the runtime thread. Use a separate thread with a deadline, or kill the child on the deadline.
- A Linux reproduction is recorded before the fix and passes after it.
- QA then reruns every slice-7 proof on macOS and the Linux VM. The pinned `charter/DONE/slice-7.md` is unchanged.

## Rejected alternatives

- Deferring the Linux hang to slice-12.
- Holding slice-7 for an owner review.
