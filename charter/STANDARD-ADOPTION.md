# COMMON-STANDARD adoption map — holden-mcp (standard v0.2)

Pinned at commit `b87c397d5e178d7f6850908f1b01d27f347a2223`, sha256 `03a2cc2c7490c6b8e135cc404b6f3a147f70ab08dee1ed1582e600ddb5f41cce`. Adopted 2026-11-04, before the first commit. Each section maps to a gate or to a bounded obligation. Obligations are work items, not exceptions.

| § | Requirement | This repository | Status |
|---|---|---|---|
| 1 | Shareable at any commit; private data outside the repository under `AGENT_PRIVATE_ROOT` | Evidence (E-REVIEW), the id map, named handoffs and the S-LENS corpus live under `$AGENT_PRIVATE_ROOT/holden-mcp/`. Committed text uses generic ids. `docs/evidence/MANIFESTS.sha256` is the public verification set. Recorded exception (owner, 2026-11-04, no expiry): the licensor name required by the license appears in `NOTICE` and in the files listed for it in `charter/safety-allow.txt`; see `charter/owner-decisions/2026-11-04-license-polyform-noncommercial.md`. | met, with the recorded licensor-name exception |
| 1 | Public-safety check on the exact candidate | `cargo xtask guard` (slice 0, P6) plus `charter safety` | obligation O-STD-1: lands in slice 0 |
| 1 | Secrets only from a secrets manager | Doppler project `holden-mcp` (`doppler.yaml`). Attestation keys sit in the isolated `attest` config, used only via `doppler run`. | met |
| 1 | Copyrighted or third-party corpora not committed | The S-LENS concept corpus comes from published design books. It is loaded from the private root and verified by a committed hash manifest. A synthetic corpus is committed for tests. The S-DOCTRINE canon is the owner's own work and is committed under `canon/holden/` (D-08). | met by design; proven in slice 0, P4 |
| 2 | Cloud-ready: fresh clone runs every gate; bind to 127.0.0.1 | Gates use only committed files: synthetic fixtures and the committed S-DOCTRINE canon (D-08). Private corpora absent → typed `canon_absent` limitation, never a pass. The MCP server uses a Unix socket or 127.0.0.1 on a configurable port. | met by design; proven in slices 0 and 2 |
| 2 | No assumed workstation | The Linux proof host (Tart VM) sits behind `xtask gate --linux-vm` with a configurable VM name. | obligation O-STD-2: a CI runner without containers before slice 1 acceptance |
| 3 | Rust default; no Python, plain JavaScript or Docker; pinned toolchain; one lockfile | `rust-toolchain.toml`, one `Cargo.lock`. `cargo xtask guard` refuses `.py`, `.js`, `.mjs`, `.cjs` and Dockerfiles. | met by design; proven in slice 0, P6 |
| 4 | README, AGENTS.md, charter/, one current-state surface | `charter/STATE.md` is the only current-state surface. Other docs point to it. | met |
| 5 | Ten cleanliness dimensions | Crate boundaries follow `docs/ARCHITECTURE.md` §3. `cargo deny` and `cargo audit` run in the gate. Forge isolation is enforced by `forge export`. | obligation O-STD-3: first full ten-dimension disposition at slice 3 |
| 6 | Exact commands; fail-without mutants; independent verifier of a different family; reopenable; own build dir per run; one heavy suite at a time | Every DONE proof names its mutant and reopen step. The verifier seat is `SEAT:codex-cli:gpt-6-luna:openai`, a different family from the Claude author. `xtask` sets a per-run `CARGO_TARGET_DIR`. | met in DONE lists; obligation O-STD-4: repository-wide suite lease in `xtask` by slice 1 |
| 7 | charter/ check, gate, freeze, receipt; charter tool pinned and built from source | The charter tool is built from source at commit `751b475` (2026-11-03). | obligation O-STD-5: record the charter pin in `xtask` and CI |
| 7 | Plan and work-contract schema pin (v5 payload root + validator release) | Not yet used | obligation O-STD-6: record the pin when this repository first emits or consumes work contracts. No home-made validator in the meantime. |
| 7 | Decisions in the repository | `charter/owner-decisions/`. Private named originals stay in the private root's `handoffs/`. | met |
| 8 | Worktree lifecycle; merge or kill; archive tags; no destructive shortcuts | No worktrees yet. The lifecycle record goes in `charter/STATE.md`. | met |
| 9 | Linked from AGENTS.md; gaps recorded | Linked from AGENTS.md. Gaps are O-STD-1 to O-STD-6 above. | met |

Attestation (2026-11-04, set up by the mister-clean seat):
- key id `holden-mcp-attest-ed25519-20261104`, Ed25519, SPKI DER sha256 fingerprint prefix `241c78e9905c4461`;
- trust policy ACTIVE (owner approved 2026-11-04) sha256 `f9d1e027ffa6df48095d6c5085d23dd023f53fe5ea6f45d35e1db129ff8839d5 (supersedes a void earlier hash; stored without trailing newline)` (actor `SEAT:codex-cli:gpt-6-luna:openai`, role `independent_qa`, valid 2026-11-04 to 2027-02-02). See owner-decisions/2026-11-04-trust-policy-activation.md.
