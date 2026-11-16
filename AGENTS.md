<!-- charter v1: harness-independent contract. Do not remove. -->
# Charter (binding for every agent, every harness)
1. Before designing, dispatching, or claiming anything done, read `charter/` in this repo: INTENT.md, GUARANTEES.yaml, LOOPS.yaml, STOP-RULES.md, DONE/<your slice>.md, STATE.md. If any is missing or stale, your first task is to produce it with the owner; do not proceed past it.
2. Done means every proof in `charter/DONE/<slice>.md` executed and its run reopened and compared. It never means "no findings". New findings become recorded obligations on the next bounded slice unless they break a guarantee listed in GUARANTEES.yaml.
3. The stop rules in `charter/STOP-RULES.md` bind you. You may not relax them. A change to them exists only as `charter/owner-decisions/<date>-<slug>.md`.
4. Every guarantee you add gets an owner, a proof and a cost. Every loop you add gets a bound, a cost per trip and a stop condition. No exceptions for "small".
5. Nothing scales before `charter.toml [slice_zero]` has `proven_on` set from a real run.
6. Before you stop, hand off, or approach a context limit: rewrite `charter/STATE.md` (proven, not proven, exact last-green and last-red commands, next three bounded items, open owner decisions). No decision may live only in a pane or transcript.
7. Run `charter check` (or `hooks/charter-check.sh`) before any commit or "ready" claim. Languages follow the owner's rule: Rust by default; no Python except an approved adapter; TypeScript only strict.
<!-- end charter -->

# holden-mcp — agent instructions

holden-mcp is a master holdout-testing system. It defines what a holdout is, derives situated cognitions from planning evidence, and generates sealed holdouts inside them. An MCP server reveals each test's private random parameters only at test time. The system measures its own effectiveness and embeds holden agents into other projects.

Read these first, in order:

1. `charter/INTENT.md`: the job, the guarantees, and the owner's mandate.
2. `charter/STATE.md`: the one current-state surface (what is proven, last green and red, next items, open decisions).
3. `docs/ARCHITECTURE.md`: the foundation decision, components and slice plan.
4. `charter/DONE/<slice>.md` for the slice you are working on.

## Binding rules

- **Common engineering standard v0.2**, pinned by commit, not copied: https://github.com/bradheitmann/mister-clean/blob/b87c397d5e178d7f6850908f1b01d27f347a2223/COMMON-STANDARD.md (sha256 `03a2cc2c7490c6b8e135cc404b6f3a147f70ab08dee1ed1582e600ddb5f41cce`). Adoption mapping and gaps: `charter/STANDARD-ADOPTION.md`. Where the standard and `charter/` disagree, the charter governs until the owner decides.
- **Languages:** Rust only (edition 2024, `unsafe_code = forbid`). No Python, no plain JavaScript, no Docker. It must build and pass on macOS and Linux.
- **Public by default:**
  - No absolute home paths, user names, host names or private project names in tracked files. Use generic ids such as S-RUNNER and S-DOCTRINE; the id map is private.
  - Private data lives under `$AGENT_PRIVATE_ROOT/holden-mcp/` (default `${XDG_DATA_HOME:-$HOME/.local/share}/agent-private/holden-mcp/`), in `evidence/`, `handoffs/`, `agent-state/`, `maps/`, `db/` and `corpora/`.
- **Secrets** come only from Doppler (`doppler.yaml`: project `holden-mcp`, config `dev`). Attestation keys live in the isolated `attest` config. No agent runs any command against `attest` (no `doppler run --config attest`, no secrets listing, no env dumps). The only permitted access is the read-only helper `attest-inspect holden-mcp`, installed under the mister-clean state directory in `$HOME/.local/state/`. Rule effective 2026-11-04, after a key leak in another repository. Never print a secret value; check its presence and length only.
- **Tests fake nothing.** No mocks, stubs or canned responses for the thing under test. Every new test or check names a fail-without mutant.
- **Each verification run uses its own `CARGO_TARGET_DIR`.**
- **Never weaken a guarantee, check or bar to make something pass.** Never pass `--no-verify`.
- **Holdout doctrine** (from S-DOCTRINE and S-RUNNER):
  - publish the why, seal the how;
  - the owner's words are the promise, and an agent never invents, widens or softens one;
  - unsure never passes;
  - authority is earned by measured error rates;
  - observe the product only at its boundary.
