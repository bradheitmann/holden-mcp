# E-HOLDEN-INTEGRATION ownership matrix (2026-11-06)

Graph: `dispatch-graph.json` (check with `jq -f validate-graph.jq dispatch-graph.json`; `valid` must be `true`).
Predecessor graph: `plans/2026-11-03-foundation/dispatch-graph.json` (slices 0–6 accepted, slice 7 built).

## Seats
| Seat | Role | Scope |
|---|---|---|
| A/EXEC-PM (Claude Code) | control plane | the plan graph, owner decisions, pushes to origin, owner liaison, escalations. Writes no product code. |
| A/DROID-MISSION-1 orchestrator (TEAM PM) | mission router | `charter/DONE/slice-{8..12}.md` (written from the node contract before dispatch, then frozen), `charter gate --dispatch/--accept`, `charter/qa-pins/slice-N.sha256`, local no-ff merges of accepted slices into main, STATE.md at acceptance, mission ledger under the private root `agent-state/mission/`. |
| A/DROID-MISSION-1 worker (DEV) | implementation | only the node's `contract.resource_scope`, on branch `slice-N`. |
| A/DROID-MISSION-1 validator (QA) | independent QA | read-only on the frozen tip; receipts `qa/slice-N/receipt-<sha8>.md`; evidence under the private root `evidence/qa/slice-N/`. Never QA's work it wrote. |
| A/HLDT-1 (Grok) | holdout author and examiner | sealed material only under the private root `holdouts/`; committed output only `plans/2026-11-06-integration/holdout-hashes.json` and `qa/holdouts/HC-N-verdict.md` (public card, hashes, verdict; no sealed content). |
| Owner | owner-only actions | `OWNER-APPROVE-DEMO` (`holden approve`), purchases, policy changes. |

## Exclusive write scopes by slice
| Slice | Files |
|---|---|
| 7 (QA only) | none; repairs only through the one remaining L-REPAIR budget, already used (c5f1903). A further defect escalates to A/EXEC-PM under S5. |
| 8 | `crates/holden-mcp/**`, `crates/holden-cli/src/**` (workflow subcommands only), tests |
| 9 | `crates/holden-cognition/**` (planning adapters) or new `crates/holden-plan-adapters/**`, `fixtures/clients/**` |
| 10 | `crates/holden-cli/**` (init, mcp relay), `crates/holden-embed/**`, `packaging/**` |
| 11 | `crates/holden-embed/tests/**`, `xtask/src/**` (e2e target only) |
| 12 | `crates/holden-eval/tests/**`, `xtask/**`, `README.md`, `docs/**`, `charter/GUARANTEES.yaml`, `charter/STATE.md` |

Slices 9 and 10 run in parallel. Both touch `crates/holden-cli` only through slice 10; slice 9 must not edit it.

## Never write
`canon/**`, `LICENSE`, `NOTICE`, `charter/INTENT.md`, `charter/STOP-RULES.md`, `charter/LOOPS.yaml`, `charter/owner-decisions/**`, any pinned `charter/DONE/slice-{0..7}.md`, `charter/qa-pins/**`, `.github/**`, any file outside this repository except the private root.

## Shared resources
- Tart VM `holden-mcp-linux`: one heavy gate at a time; delete `/tmp/holden-mcp-*` and `~/holden-mcp-gate` on the VM after each slice. Never start, stop or reconfigure the VM; ask A/EXEC-PM.
- `/tmp/holden-mcp-*` on the Mac: delete your own scratch when the slice is accepted.
- No launchctl, systemd, Docker, Python, or inference-server control.
