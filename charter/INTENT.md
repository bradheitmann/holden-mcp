# Intent card — holden-mcp (master holdout-testing system)

> **Status: CONFIRMED by the owner on 2026-11-04**: "I do approve your intent file … I approve your plan, so go ahead and proceed." The owner also noted "There are some other things that need to happen"; those are pending and will be recorded as decisions when received. Drafted 2026-11-03 from the owner's mandate.
>
> Source names in this repository are generic ids (S-RUNNER, S-DOCTRINE, S-LENS, S-FORGE, S-CANON-MCP), per the common engineering standard §1. The id map is private: `$AGENT_PRIVATE_ROOT/holden-mcp/maps/sources.yaml`.

- Underlying job (not the symptom): Make holdout testing a portable instrument that any agent-built project can carry. It defines what a holdout is and where it comes from. It generates tests from the people and agents who will actually meet the product. It keeps part of every test unknowable to the builder until the moment the test runs.
- Beneficiary: The owner, across many agent-built projects. Behind the owner: the people and agents those products serve, meaning human, agent, seam, and the unknown future participant.
- Situation: Coding agents pass the tests they can see and miss what the work was for. There are three partial answers today. S-RUNNER is a sealed, deterministic Linux runner. S-DOCTRINE has the doctrine, research and taxonomy but weak tooling. S-LENS has randomized design lenses but nothing sealed. None of them generates situated tests, measures its own effectiveness, or embeds in other projects.
- Outcome (observable): From one project's planning documents, `holden` produces approved situated cognitions. It generates sealed holdouts set inside them and serves them through an MCP server. At test time it adds a private random element that is revealed only through that server and can be verified afterwards. It runs the holdouts against the real product on macOS and Linux and records a signed, replayable verdict. It measures which cognitions and lenses catch real defects, and it installs a separate holden agent into another project with one command.
- Non-goals: No Python. No Docker. No weakly typed code (Rust; strict TypeScript only where a host demands it). No UI platform. No human in the check path. No agent inventing or widening a promise; the owner's words stay the promise. No code copied from S-RUNNER until it carries a license.
- The 3 guarantees that matter: G-UNPREDICTABLE, G-SITUATED, G-MEASURED. Their rows are in GUARANTEES.yaml, with G-PORTABLE added for the mandate's platform constraint.
  1. G-UNPREDICTABLE: before a test runs, a builder cannot learn its random parameters, even with read access to the sealed plan. After the test, anyone can verify the parameters were not chosen to steer the outcome.
  2. G-SITUATED: every generated holdout traces to an owner-approved promise and to a situated cognition derived from planning evidence and the canon, with provenance on each link.
  3. G-MEASURED: every lens, cognition and check carries a measured catch rate and false-pass rate on known controls. Its authority to block is computed from the pessimistic bound, never assumed.
- Largest uncertainty: Whether the seal holds when the implementer agent and the sealed side share one OS user on a laptop, which is the common embedded case. The design answers this by binding randomness to the evaluated commit and to a server-held key. That still needs proof.
- Premise check — symptom named by the requester: "a master holdout testing MCP repo". Job it actually serves: tests that agents cannot game, grounded in who the product is for, and portable across every project the owner runs.
- Evidence lines: the five sources below. Private evidence is referenced by hash only.
  - The owner's mandate of 2026-11-03, quoted below.
  - E-REVIEW: a formal review of S-RUNNER. It is held privately; its manifest hash is in `docs/evidence/MANIFESTS.sha256`. It contains 90 remediation items, 21 hybrid features and 63 verified claims.
  - Method analyses of S-RUNNER, S-DOCTRINE and S-LENS (2026-11-03).
  - S-FORGE: a domain-forge module, version 0.3.0.
  - S-CANON-MCP: canon pin `skill-v2.3.0 adapter-v0.2.0`.

## The owner's mandate (2026-11-03)

Quoted verbatim except for one bracketed redaction of a private project name. The unredacted text is in the private handoffs.

> define what holdout testing is including the philosophical roots and how these generate the synthetic users / situated cognitions we need to test for, create a holdout test generator capable of generating random tests set inside those situated synthetic cognitions, overlay domain-specificity (see how [a domain-forge project] is implementing the domain-specific expert agent constitutional system prompt forge -- it's still a work in progress but good patterns and the source forge is worth copying or using as a pattern so long as you keep it modular and separate from the rest of the codebase i need to be able to export it into other projects a clean forge directly from your repo), do recursive feedback and telemetry measuring your effectiveness, and embedding holden agents as sepearate entities inside of other projects that implement this and use holden to produce just in time holdout testing in any situation combining what is known in the planning to create the situated cognitions but at the moment of the test producing an element of randomness that, even with the situated cognition being a sha sealed or otherwise encrypted test contains yet another private unknown element which only becomes known by hitting the mcp server to reveal the randomization parameters.

> No python. no docker. use only strongly typed. it has to be compatible with macos and linux.
