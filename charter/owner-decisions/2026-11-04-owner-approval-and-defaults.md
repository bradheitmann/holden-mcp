# Owner decision — 2026-11-04 owner-approval-and-defaults

approved_by: owner

## Context

On 2026-11-03 the orchestrator seat proposed a foundation decision (a new Rust repository) and an architecture (`docs/ARCHITECTURE.md`). The proposal had seven owner decisions, D-01 to D-07, each with a default, plus the draft intent card `charter/INTENT.md`.

## Decision

Owner, 2026-11-04, verbatim: "I do approve your intent file. There are some other things that need to happen. Other than that, I approve your plan, so go ahead and proceed."

Owner, 2026-11-04, verbatim: "Go ahead and use Tart VM on this Mac."

- D-01: the intent card is confirmed.
- D-04: the Linux proof host is a Tart VM on the development Mac (`holden-mcp-linux`, from the cirruslabs Ubuntu image, 6 CPU / 8 GiB / 40 GB). No containers.
- Approving the plan adopts the stated defaults:
  - D-02: name `holden-mcp`, private repository on the owner's account, created when first needed.
  - D-03: clean-room with respect to S-RUNNER until it is licensed.
  - D-05: post-verdict parameters go to auditors and the owner only.
  - D-06: vendor the S-FORGE methodology with `UPSTREAM.md` provenance, subject to licensing confirmation.
  - D-07: MIT.

## Rejected alternative

- Forking S-RUNNER: no license, Linux-only evaluation, Docker-based proofs, different scope.
- Extending S-DOCTRINE: shell and plain JavaScript violate the strong-typing mandate.
- Lima or a remote rig as the Linux host: the owner chose Tart.

## Follow-ups

- "Some other things that need to happen": pending from the owner. They will be recorded as new decisions.
- The common engineering standard was pinned on 2026-11-04 at the owner's request, relayed by the mister-clean seat. Doppler project `holden-mcp` was created with an isolated `attest` config (key id `holden-mcp-attest-ed25519-20261104`, SPKI fingerprint prefix `241c78e9905c4461`). The trust policy draft has sha256 `f9d1e027ffa6df48095d6c5085d23dd023f53fe5ea6f45d35e1db129ff8839d5 (supersedes a void earlier hash; stored without trailing newline)`, with independent QA actor `SEAT:codex-cli:gpt-6-luna:openai`. It stays inactive until the owner approves it.
- D-06: confirm S-FORGE upstream licensing before slice 6 dispatch.
- D-08 (new, open): commit the S-DOCTRINE canon files, or load them from the private root. The default is the private root.
