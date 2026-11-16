# Control-plane ruling — 2026-11-06 w2-role-semantics

decided_by: A/EXEC-PM (architecture inside the owner's approved intent; no stop rule relaxed)

## Context

Slice-11 tested holden on a copy of a real repository and surfaced defect B4. An outside agent reaches holden through `/holden` with the implementer role. W2 (check) returned `workflow_unavailable` there, because the signed evaluator ledger is attached only to the evaluator-side config. W4 (status) has the same evaluator-only boundary. The worker correctly refused two shortcuts: using the evaluator socket, and handing the ledger to the implementer.

## Decision

W2 and W4 are composed server-side. The server reads the ledger and the sealed plan from its own sealed config, and runs or reads them on the caller's behalf. The caller stays implementer:

- It may trigger a just-in-time run on a commit and receive the verdict with a consequence-level diagnostic.
- It may read aggregate telemetry: catch rates with sample sizes, and authority levels.
- It never receives moment-of-test parameters, sealed plan contents, or the ledger key.

`reveal_params` and `audit_reveal` stay evaluator-only, and the existing refusal tests still bind.

This is the slice-8 design, under which workflow tools compose lower-level tools server-side. It matches the intent that the private element is revealed only through the MCP server and never to the coding agent.

Plan rev 4 widens slice-11's write scope to `crates/holden-mcp/src/workflow*.rs` and the config-assembly code in `crates/holden-embed/src/embed.rs`. This covers B3 and B4.

## Also decided

- Slice-12 P4, owner-key custody: write scope widened to `crates/holden-seal/**` and `crates/holden-embed/src/approve.rs`, for an OS key-store backend. Approvals already made must still verify against `owner.pub`.

## Rejected alternatives

- Routing W2 through a request to the separate holden agent process. That adds a second inter-process protocol for the same guarantee.
