# Owner decision — 2026-11-06 mission-qa-seat

approved_by: owner

## Context

Slices 0–6 were QA-accepted by a Codex seat (gpt-6-luna). The owner then gave the remaining work, slice-7 QA and slices 8–12, to one Factory Droid mission seat, A/DROID-MISSION-1, to save tokens.

## Decision

Owner, 2026-11-06, verbatim: "for all dev and qa work hand it to A/DROID-MISSION-1 which will run a 3 agent workflow including an orchestrator (like a TEAM PM), a worker (eg a DEV) and a verifier (e.g. a QA)" and "I've also provided A/HLDT-1 so you can perform holdout testing".

So, from slice-7 QA on:

- The mission validator is the QA seat. It runs MiniMax M3.1 Flash; the worker runs DeepSeek V4.1 Flash, a different model family.
- Each QA receipt is a `charter.receipt/1` file and names the validator model. A receipt that names the worker's model is refused.
- Independent holdout checkpoints HC-1, HC-2 and HC-3, run by A/HLDT-1 on Grok, gate the next frontiers. They add a third model family the build pod never sees.
- Earlier receipts stay valid. "A Codex receipt" in the completion criteria means the independent QA seat in force when the slice was accepted.

## Rejected alternative

Keeping Codex as QA for slices 7–12. The owner chose the mission seat to save tokens.
