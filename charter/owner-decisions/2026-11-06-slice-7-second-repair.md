# Owner decision — 2026-11-06 slice-7-second-repair

approved_by: owner

## Context

The slice-7 Linux VM gate hung for three hours at c5f1903. The original run at dc832b8 hung the same way.

The cause is in the test, not the product. `holden-embed` test `host_implementer_cannot_reveal` still holds the write end of the `holden connect` child's stdin after `client.cancel()`. The stdio bridge therefore never sees end-of-input and never exits, and the test blocks. Evidence from /proc on the VM: the test's fd 22 was the write end (O_WRONLY) of the bridge's stdin pipe, and the bridge was waiting in `pipe_wait_readable` and `unix_stream_data_wait`. macOS passes.

Slice-7 had already used its one L-REPAIR cycle (c5f1903), so stop rule S5 sent the slice to the owner.

## Decision

The owner chose "Allow one more fix" (2026-11-06): one more repair cycle for slice-7.

Scope is limited to `crates/holden-embed/tests/embed.rs`:
- close or kill the bridge child after use;
- bound every stdio MCP exchange in the slice-7 tests with a timeout that fails rather than hangs.

QA then reruns every slice-7 proof on macOS and the Linux VM. The pinned `charter/DONE/slice-7.md` is unchanged.

This does not relax S5. It is the owner deciding the escalated slice, as S5 requires.

## Rejected alternatives

- Accepting slice-7 on macOS only and folding the fix into slice-8. That would weaken G-PORTABLE.
- Holding all work for an owner review.
