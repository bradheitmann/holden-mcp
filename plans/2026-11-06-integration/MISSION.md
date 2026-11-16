# MISSION M-HOLDEN-INTEGRATION (A/DROID-MISSION-1)

Run this mission until node `HOLDOUT-HC3` is accepted or a stop rule fires. Do not stop between slices. When a frontier waits on something outside the pod (an owner node, a holdout checkpoint), prepare the next frontier's DONE lists and QA contracts, then keep polling the mission ledger. Do not end the mission.

## Inputs (read in this order)
1. `plans/2026-11-06-integration/dispatch-graph.json`: the nodes, their contracts, and the frontiers. A node may start only when everything in its `depends_on` is done.
2. `plans/2026-11-06-integration/OWNERSHIP.md`: who writes what. Writing outside a scope is a stop (S-rule breach).
3. `charter/INTENT.md`, `charter/STOP-RULES.md`, `charter/LOOPS.yaml`, `charter/STATE.md`, `docs/ARCHITECTURE.md`.
4. `charter brief orchestrator` (orchestrator), `charter brief worker` (worker), `charter brief verifier` (validator).
5. `charter/DONE/_template.md` and the pinned `charter/DONE/slice-7.md` on branch `slice-7` as the house style for DONE lists.

## Roles inside the pod
- **Orchestrator (TEAM PM).** Walks the graph frontier by frontier. For each DEV node:
  - Before any implementation, writes `charter/DONE/slice-N.md` from the node's `contract`: one numbered proof for each acceptance item, each with a reopen step and a named fail-without mutant.
  - Commits that file on branch `slice-N`, cut from the base the contract names.
  - Runs `charter gate slice-N --dispatch`.
  - Hands the worker only the node contract and the DONE list.
- **Worker (DEV).** Implements inside `resource_scope` only. Strict Rust, no `unsafe` unless the slice's DONE list allows it, no Python, no Docker, no JavaScript. Finishes with both gates green, then hands the orchestrator the tip SHA:
  - `cargo xtask gate slice-N`
  - `HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-N --linux-vm` (test names and pass counts must match the host)
- **Validator (QA).** Works in a fresh context on the frozen tip: the orchestrator runs `charter freeze slice-N` first.
  - Reopens every proof on macOS and on the VM.
  - Runs every named mutant and records that it fails.
  - Runs `charter check --strict` and the guard.
  - Writes a `charter.receipt/1` receipt to `qa/slice-N/`, naming its own model (MiniMax M3.1 Flash), and puts raw evidence in `$AGENT_PRIVATE_ROOT/holden-mcp/evidence/qa/slice-N/`.
  - Never validates code it wrote, and never edits product code.

## Acceptance and landing (orchestrator)
1. Validator receipt `accepted` → `charter gate slice-N --accept` → write `charter/qa-pins/slice-N.sha256`, following the slices 0–5 pattern.
2. Merge `slice-N` into local `main` with `--no-ff` (message `merge: slice-N accepted (<title>)`), then commit `docs(charter): accept slice-N`, updating STATE.md as the slice-5 commit did.
3. Never push and never force anything. A/EXEC-PM pushes.
4. Append one line to `$AGENT_PRIVATE_ROOT/holden-mcp/agent-state/mission/ledger.md`: `<UTC time> <node> <state> <sha> <note>`. States:
   - DISPATCHED
   - DEV_DONE
   - QA_ATTEMPT_n
   - ACCEPTED
   - REJECTED
   - REPAIR
   - BLOCKED
   - WAITING_HOLDOUT
   - WAITING_OWNER
   - DONE

## Budgets and stop rules
- L-QA allows 3 QA attempts per slice. L-REPAIR allows 1 repair cycle per slice. A rejection with the repair budget used is a stop: S5 escalates to A/EXEC-PM.
- Slice-7 has already used its repair cycle (c5f1903). If slice-7 QA finds a code defect, mark it BLOCKED and escalate. Do not repair it.
- Stop rules S1–S7 bind every pod member. When one fires, write `BLOCKED <reason>` to the ledger and keep working on any frontier the block does not touch.

## Holdout checkpoints (HC-1, HC-2, HC-3)
- These are sealed holdouts authored and run by A/HLDT-1, a separate seat. The pod never reads `$AGENT_PRIVATE_ROOT/holden-mcp/holdouts/` and never asks for its contents.
- When a checkpoint's dependencies are all ACCEPTED, write `WAITING_HOLDOUT HC-n <main sha>` to the ledger and create `agent-state/mission/HC-n.ready` containing the main SHA.
- A/HLDT-1 writes `qa/holdouts/HC-n-verdict.md` and `agent-state/mission/HC-n.verdict` (`ACCEPTED` or `FAILED <public consequence>`).
- ACCEPTED: continue with the next frontier.
- FAILED: the failure names a consequence, not a method. The orchestrator turns it into a repair node for the slice it implicates (one repair cycle) and runs the checkpoint again.

## Owner node
`OWNER-APPROVE-DEMO` blocks only SLICE-07-QA proof P4.
- Run SLICE-08 meanwhile.
- Write `WAITING_OWNER OWNER-APPROVE-DEMO` once.
- A/EXEC-PM gets the owner to run `holden approve`, then writes `agent-state/mission/OWNER-APPROVE-DEMO.done`.

## Shared machine rules
- Use one heavy gate at a time.
- Delete `/tmp/holden-mcp-*` scratch after each accepted slice, on the Mac and the VM.
- Use `PATH="$HOME/.cargo/bin:$PATH"` (pinned toolchain 1.95.0).
- Use `< /dev/null` for non-interactive tools.
- Never use launchctl, systemd, sudo, VM start/stop, or inference-server control.
- Never print secrets.
- Never write the owner's username or email into a committed file. Private paths go only in `$AGENT_PRIVATE_ROOT`.

## Definition of mission done
- Nodes SLICE-07-QA through SLICE-12-QA are ACCEPTED and merged into local main.
- `agent-state/mission/HC-3.verdict` reads ACCEPTED.
- `charter check --strict` is green on main.
- The ledger's last line is `DONE`.
