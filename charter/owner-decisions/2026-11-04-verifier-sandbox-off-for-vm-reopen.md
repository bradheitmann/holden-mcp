# Owner decision — 2026-11-04 verifier-sandbox-off-for-vm-reopen

approved_by: owner

## Context

Slice-0 QA attempt 1 was run by the independent verifier seat `SEAT:codex-cli:gpt-6-luna:openai` under the `workspace-write` sandbox. It rejected the slice because two reopenings were impossible inside that sandbox:

- D1: the sandbox blocked `tart ip`, so the Linux VM was unreachable.
- D2: the verifier could not fetch the hosted canon lock.

Attempt 2 was started with `--dangerously-bypass-approvals-and-sandbox`, which gives full access to the development Mac. Its prohibitions were stated in the prompt only:

- no change to the repository checkout;
- no Doppler access;
- no `.py` or plain JavaScript files;
- no Docker;
- no VM reconfiguration.

The orchestrator told the owner the exact configuration:

| Setting | Value |
|---|---|
| CLI | codex-cli 0.163.0 |
| Model | gpt-6-luna |
| Provider | `openai`, OpenAI-hosted inference |
| Login | the owner's ChatGPT login |
| Reasoning effort | medium |
| Approvals | never |
| Sandbox | `danger-full-access` |

The orchestrator also stated that everything the verifier reads is sent to OpenAI. It offered a sandboxed alternative: add a VM IP override to `xtask` and re-run under `workspace-write`.

## Decision

Owner, 2026-11-04, verbatim: "Very well, continue." and then "Continue."

## Rejected alternative

Stop attempt 2, add a VM-address override to `xtask` (a new frozen candidate), and re-verify under `workspace-write`. The owner chose to continue instead.

## Follow-ups

- Obligation for the slice after slice-0: `xtask gate --linux-vm` accepts an explicit VM address (`HOLDEN_VM_ADDR`), so that future verifiers can stay sandboxed.
- Verifier prompts that leave the sandbox off must keep the explicit prohibition list.
- Obligation for the slice after slice-0, found in QA attempt 2: `xtask gate slice-0 --linux-vm` did not forward the per-run target-directory base to the VM run. The verifier ran the VM gates directly with their own target directories.
