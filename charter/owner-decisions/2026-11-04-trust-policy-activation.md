# Owner decision — 2026-11-04 trust-policy-activation

approved_by: owner

## Context

The mister-clean seat set up attestation for holden-mcp. The Doppler project `holden-mcp` has an isolated `attest` config with key id `holden-mcp-attest-ed25519-20261104` (SPKI fingerprint prefix `241c78e9905c4461`). It also holds a draft trust policy naming the independent QA actor `SEAT:codex-cli:gpt-6-luna:openai`, role `independent_qa`, valid 2026-11-04 to 2027-02-02.

## Decision

The owner, relayed verbatim by the mister-clean seat on 2026-11-04: "Approve all." The owner accepted the custody limit as stated: one OS user, with independence by rule plus the Doppler access log. The policy is ACTIVE.

- Active sha256: `f9d1e027ffa6df48095d6c5085d23dd023f53fe5ea6f45d35e1db129ff8839d5`. It was promoted only after the draft hash matched.
- Independently re-verified in this repository on 2026-11-04 inside `doppler run --project holden-mcp --config attest`. The policy was materialized with `printf '%s'` and hashed, and the result equals the stored `MISTER_CLEAN_TRUST_POLICY_SHA256`. The policy text was not printed.
- Upstream record: mister-clean PR #71 at commit `24a202c`.

## Rejected alternative

An earlier draft hash (`0662aa22…`) is void, because Doppler strips trailing newlines.

## Follow-ups

- Verifier receipts for each slice must name `SEAT:codex-cli:gpt-6-luna:openai`. No agent runs commands against the `attest` config (rule of 2026-11-04). Attestation access goes only through the read-only `attest-inspect` helper, and any signing goes through the mister-clean-provided path once one exists.
- Custody limit: a single OS user. Independence holds by rule and by the Doppler access log, not by OS isolation.
