# holden-mcp — foundation decision and architecture

Status: approved by the owner on 2026-11-04 (`charter/owner-decisions/2026-11-04-owner-approval-and-defaults.md`). Source names are generic ids per the common engineering standard §1. The id map is private (`$AGENT_PRIVATE_ROOT/holden-mcp/maps/sources.yaml`).

| Id | Generic description |
|---|---|
| S-RUNNER | A third-party Rust holdout runner (private, no license declared) |
| S-DOCTRINE | The owner's original holdout doctrine and research repository |
| S-LENS | The owner's randomized design-lens skill |
| S-FORGE | A domain-forge module in another of the owner's projects |
| S-CANON-MCP | The owner's hosted holdout-canon MCP server |
| E-REVIEW | The 2026-11-03 formal review of S-RUNNER (private evidence; manifest hash in `docs/evidence/MANIFESTS.sha256`) |

## 1. Foundation decision

**Decision: a new Rust repository (this one).**
- S-DOCTRINE and S-LENS enter as hash-pinned canon and corpora.
- S-RUNNER is a reference design and an optional execution backend, not the base.

| Option | Why not the base |
|---|---|
| Fork S-RUNNER | 1. **No license.** No license is declared, so forking or copying is not clearly permitted (E-REVIEW R-018). 2. **Linux-only.** Its evaluate path has no macOS seam (R-060). 3. **Docker.** Its release and wall proofs run in Docker. 4. **Different scope.** Its version 1 runs supplied holdouts and does not generate them. 5. **Open defects.** It has 11 P0 and 79 other open remediation items. |
| Extend S-DOCTRINE | It is shell and plain JavaScript, which violates the strong-typing mandate, and the owner rates it "not well built". Its value is doctrine and research, which enter here as data. |
| **New repo** | Strong typing (Rust). macOS and Linux from day one. No Docker. PolyForm Noncommercial 1.0.0 (D-07). A separable forge. Traceable provenance. |

What each source contributes:

| Source | Enters as | How |
|---|---|---|
| S-DOCTRINE | Canon: doctrine, the 65-code failure taxonomy, the encounter-profile model, claim-registry rules, the Nothing Doctrine, three trust axes, constituencies, philosophical roots | The four canon files are pinned by SHA-256 in `canon/CANON-PIN.txt`, matching S-CANON-MCP (`skill-v2.3.0 adapter-v0.2.0`). The four files are committed under `canon/holden/` (D-08) and `holden canon verify` checks them against the pin by default. Typed loaders live in `holden-canon`. |
| S-LENS | Lens and cognition corpus: design concepts and situated cognitions | It is derived from published design books, so it is **never committed**. It is loaded from `$AGENT_PRIVATE_ROOT/holden-mcp/corpora/lens/` and verified against a committed manifest. A small synthetic corpus is committed for tests. Known defects are fixed as recorded deltas: unseeded draws, timestamped ids, one stereotyped cognition. |
| S-RUNNER | Design only, clean-room: public card / sealed plan split; unsure never passes; earned authority; rung model; verdict vocabulary; observe at the boundary | A compatibility emitter lets S-RUNNER execute our plans on Linux. Its code is reused only after it is licensed (D-03). |
| S-FORGE | The pattern for `modules/holden-forge/` | A self-contained module with `VERSION`, `UPSTREAM.md`, `SHA256SUMS` and a round-trip export implemented in Rust. S-FORGE is the owner's proprietary work; the module carries it under this repository's license (D-06). |
| E-REVIEW | Design constraints and regression controls | Each P0 defect in S-RUNNER becomes a "never again" control in §3.5. |

## 2. What holdout testing is (the canon crate's job)

- **Definition.** A holdout is a test the builder never sees. It asks whether the work served the person or agent it was for: "hospitality compiled into evidence", and "publish the why, seal the how".
- **Philosophical roots, as typed records citing canon sections:**
  - hospitality and ethics before ontology (the unassumed Other);
  - situated cognition (situations, not personas);
  - regenerative loops;
  - measured reliability;
  - structural independence.
- **Doctrine decision (made in slice-3, recorded in slice-4):** the regenerative-loop root cites the canon spec's Section 8, THE LEARNING LOOP (`canon/holden/holdout_skill_spec_v2_3_0.md`, heading `## Section 8: THE LEARNING LOOP`). The canon never uses the word "regenerative"; its learning loop (update cadences, post-fix verification, obsolescence triggers) is the canon's own statement of this root, so the citation stands on that section's bytes.
- **Each root is a generator, not a quote:**
  - Roots yield **constituencies**: human, agent, seam and successor.
  - Constituencies × **circumstance axes** span the space of situated cognitions. The axes are attention, time pressure, interruption, device, permission, prior knowledge, hostility and handoff.
  - The Nothing Doctrine yields absence probes.
  - The trust axes yield permission pairs.

## 3. Architecture

Rust workspace, edition 2024, `unsafe_code = forbid`, exact pins, with `cargo deny` and `cargo audit` in the gate. Targets: aarch64/x86_64 Apple Darwin and Linux GNU. There is no Docker. Linux proofs run in a Tart VM on the development Mac (D-04).

```
holden-mcp/
├─ charter/                 project contract
├─ canon/                   CANON-PIN.txt, holden/ (committed S-DOCTRINE canon, D-08) ; corpus manifests
├─ modules/holden-forge/    SELF-CONTAINED, exportable forge (own Cargo workspace)
├─ crates/
│  ├─ holden-canon          typed doctrine, taxonomy, constituencies, circumstance axes, pin verification
│  ├─ holden-cognition      planning evidence → candidate situated cognitions → owner approval
│  ├─ holden-gen            promise × cognition × lens → sealed plan
│  ├─ holden-seal           AEAD sealing, commitments, VRF draw + proof, key stores
│  ├─ holden-eval           rung-0 checks, boundary drivers, confinement backends
│  ├─ holden-ledger         append-only signed ledger, telemetry, effectiveness
│  ├─ holden-mcp            MCP server (rmcp): the sealed oracle
│  ├─ holden-embed          installs a holden agent into another project
│  └─ holden-cli            `holden` binary
├─ xtask/                   gates, guard, Linux-VM gate driver
└─ docs/                    architecture; evidence manifests (hashes only)
```

The dependency direction is `canon ← cognition ← gen ← seal`. `eval` and `ledger` stand alone, and `mcp` and `embed` compose the rest. The forge is a file contract only: no crate imports it.

### 3.1 Situated cognition (G-SITUATED)

A `SituatedCognition` has these fields:
- `constituency`
- `role`
- `circumstance` (typed axes)
- `cannot_assume`
- `goal`
- `pressure`
- `evidence[]`
- `approved_by`

Its id is `H(canonical bytes)`. The pipeline:
1. Planning documents are parsed into promises, quoted verbatim.
2. Canon generators propose cognitions.
3. The forge pack supplies the domain overlay.
4. The owner approves.

Only approved cognitions host tests. A lens suggestion that implies a new promise goes to "needs owner words" and never becomes a check.

### 3.2 Three-layer secrecy and the MCP reveal (G-UNPREDICTABLE)

| Layer | Known when | Readable by | Mechanism |
|---|---|---|---|
| L1 Situated cognition | Planning | Public (the *why*) | Content hash, owner-approved |
| L2 Sealed plan | Generation | Sealed side only | XChaCha20-Poly1305 under a sealed key; the builder sees the public card and `plan_commit = H(ciphertext)` |
| L3 Moment-of-test parameters | Run time only | Evaluator during the run; auditors and owner after the verdict (D-05) | `params = VRF_sk(plan_commit ‖ evaluated_commit ‖ server_nonce ‖ builder_nonce)` |

L3 depends on the evaluated commit, which does not exist until the builder commits, and on a key the builder never holds. So a leak of L2 still reveals no parameters.

The VRF admits exactly one valid output per input, so the server cannot grind for favourable parameters. The server nonce is committed before the builder commits, so the builder cannot grind either. After the verdict, `audit_reveal` returns the parameters and the proof, which anyone can verify against the published epoch key.

Crates: `schnorrkel` (sr25519 VRF), `chacha20poly1305`, `hkdf`, `sha2`, `ed25519-dalek`.

Key stores sit behind a trait:
- macOS Keychain;
- the Linux kernel keyring, or a 0600 file owned by the sealed user;
- Doppler, for attestation keys only.

MCP tools:

| Tool | Caller and condition |
|---|---|
| `brief` | Anyone |
| `commit_nonce` | Server |
| `begin_run` | Evaluator |
| `reveal_params` | Evaluator only, during the run |
| `submit_observation` | Evaluator |
| `verdict` | Evaluator |
| `audit_reveal` | Anyone, after the verdict |
| `telemetry` | Anyone |

Roles are authorized by socket peer credentials (`getpeereid` on macOS, `SO_PEERCRED` on Linux, via `rustix`), or by role-bound tokens when the server is remote. Local services bind to `127.0.0.1` or a Unix socket, never `0.0.0.0` (standard §2).

### 3.3 Generator

`holden-gen` builds a sealed plan from a promise, an approved cognition and a lens draw. A plan contains:
- encounter steps written from inside the cognition;
- a deterministic check per step;
- an absence probe;
- a permission pair;
- a targeted broken variant used as a mutation control.

Generation is deterministic given its inputs. All randomness sits in L3. Output is native, plus the S-RUNNER compatibility emitter.

### 3.4 Evaluation without Docker (macOS + Linux)

The evaluator observes only at the boundary: process, file, socket, HTTP and SQLite. Confinement is a trait with three implementations:

| Platform | Confinement |
|---|---|
| Linux | A dedicated world user, Landlock, a network namespace when privileged, rlimits, and cgroup v2 when available |
| macOS | A dedicated world user, a `sandbox-exec` profile and rlimits |
| Personal mode | The same user with the cryptographic seal only, labelled as weaker in every verdict |

Slice-5 adds violation observation and typed limitations:
- **macOS product runs:** a second profile kills any process that tries to write outside its world or use the network. Such a run fails the verdict with the typed reason `confinement_violation`.
- **Linux:** Landlock denies these operations silently, so attempts are not observed.
- **Typed limitations:** what a backend cannot confine or observe is recorded in the verdict:
  - `reads_not_confined` (both backends);
  - `udp_not_restricted` and `violation_attempts_not_observed` (Linux);
  - `child_violations_not_observed` (macOS product runs);
  - `violation_attempts_not_observed` (macOS builds).
- **Authority cap:** each of these limitations caps authority at recommend.

Slice-7 confines reads, so a product or a build script cannot read the private root, a key store or the sealed side:
- **macOS product runs:** `file-read*` outside the world, the launcher and the system paths a process needs kills the process, and so does starting a child process (`process-fork`). The read and child limitations are gone, so a macOS verdict carries no limitation and its checks can earn `block`.
- **macOS builds:** reading a file's data outside the world, the toolchain, the developer directory and the system paths kills the offending process, as do writes and network, which fails the build and so the verdict. File metadata stays visible to builds, because the toolchain resolves its own paths. Two narrow allowances keep the system C toolchain working: `xcrun`'s `xcrun_db*` cache in the user's temporary directory and the `.CFUserTextEncoding` file. The evaluator resolves the SDK and the C linker once, outside confinement, and builds run with incremental compilation off.
- **Linux:** Landlock gets a read allowlist: the world, the system paths and, for builds, `/proc`, `/sys` and the toolchain. It still denies silently, so Linux keeps `udp_not_restricted` and `violation_attempts_not_observed`.
- **Self-check:** the launcher must fail to list the world's parent directory. If it can, the run records `reads_not_confined`, never a silent pass.

### 3.5 Never-again controls from E-REVIEW

Each of these is a seeded mutant in G-MEASURED:
- the implementer's environment is never applied to a privileged process;
- the evaluated commit's tree is scanned for canaries;
- config changes are always diffed and reported;
- limits are enforced, not just computed;
- every record stores its replay inputs and rule versions;
- verdicts are signed;
- world ids are scoped per evaluation.

### 3.6 Recursive feedback and telemetry (G-MEASURED)

Every run appends a signed ledger record containing the cognition, lens ids, parameter proof, observations, verdict and timing.

Controls are sealed known-broken and known-good builds. From them the ledger measures:
- catch rate;
- false-pass and false-fail rates, with Wilson bounds;
- lens yield;
- cognition yield;
- time to repair.

These measures feed back in four ways:
- **Authority:** advise, recommend or block, taken from the pessimistic bound.
- **Draw weights:** weights are data, and the VRF still picks.
- **Re-planning triggers.**
- **Self-test:** the system runs holdouts against its own code.

As built in slice-5 (`crates/holden-ledger`):
- **Ledger:** one SQLite file in the private root's `db/`. Records are chained by SHA-256 and signed with Ed25519. An append checks only the tip, and `holden ledger verify` checks the whole chain, naming each record that does not hold. Truncation of the newest records is caught only against a tip recorded elsewhere (`--tip`). Accepted verdicts are appended by the MCP server when its sealed config names a ledger.
- **Controls:** these are fixture commits run through the real evaluator. Known-good controls are `keeps`, `keeps-alt` and `reconfig`. Known-broken controls are the fixture mutants, the do-nothing variant of each probe's promise (one of them is the plan's targeted broken variant), and the never-again controls: confinement escapes, a hang, and a commit carrying the plan's canary. A known-broken control counts only against plans that check its target. The holdout is a promise and cognition pair, and its version is bumped by re-plan items.
- **Rates and authority:** catch and false-fail rates are kept per check, lens concept, cognition, plan and integrity rule, with 95% Wilson bounds. Authority comes from the bounds only, against bars in the sealed server config. The defaults are block at catch ≥ 0.90 and false-fail ≤ 0.05, and recommend at ≥ 0.50 and ≤ 0.25, with at least 10 broken and 10 good trials. Any typed confinement limitation caps authority at recommend.
- **Draw weights:** each lens's weight is `max(1, round(4 × corpus weight × (caught + 1) / (trials + 2)))`.
- **Triggers:** these are a canary leak in a real evaluation, a pass on a do-nothing product, and a control that proves a verdict false.
- **Not built yet:** time to repair.

### 3.7 Domain overlay: `modules/holden-forge`

The module follows the S-FORGE pattern:
- **Methodology:** domain forge, synthesis and agent forge.
- **Domain pack:** a holdout-engineering pack.
- **Agent constitutions:** planner, sealer, evaluator and adversary.
- **Rust `forge` binary:** `pack`, `constitution`, `views`, and `export <dir>`. `export` regenerates `SHA256SUMS` and refuses any absolute path or workspace reference.
- **Isolation:** the module has its own Cargo workspace and no dependency outside its directory.

### 3.8 Embedding holden agents

`holden embed <project>` writes these into the host project:
- `.holden/config.json`
- `.holden/agents/holden.md` (forged for the host's domain)
- `.holden/cognitions/` (public halves)
- the MCP client config for the host's harness
- a post-commit hook for just-in-time runs

The holden agent is a separate entity. It has its own constitution and process, and talks to the implementer only through briefs and fixed diagnostics. The implementer's role cannot call `reveal_params`.

As built in slice-7 (`crates/holden-embed`):
- **Embed:** `holden embed <project>` writes the public run file, the constitution, one cognition's public half per promise, a `holden` entry in the host's `.mcp.json` (the command `holden connect --role implementer --socket <implementer socket>`) and a two-line `post-commit` shim (`exec holden jit "$@"`). The constitution is forged through `forge export`, `forge verify`, the export's pack with its `domain` set to the host's, `forge pack` and `forge constitution --role evaluator`. Keys, sealed plans, server configs, the ledger and runs live in the agent's state, by default `<private root>/hosts/<host name>`. Embed is idempotent, and it never replaces a post-commit hook that is not its own.
- **Approval:** `holden approve` is the owner's command. It signs each (cognition, promise) pair with the owner key from the owner's key store and publishes the owner's public key there. Admission trusts only that published key, never a key named in a record or in the host.
- **Agent:** `holden agent serve` is the agent's own process. It admits only approved cognitions, seals one plan each, and serves one server core on two sockets, each with its own sealed role map: its own (evaluator) and the host's (implementer). It then commits a nonce for every plan, recording the host's HEAD.
- **Just in time:** `holden jit`, from the hook, runs each plan on the new commit. It uses a nonce armed at a proper ancestor of the commit, `begin_run` on the commit's SHA (which must consume that commitment), `reveal_params` (verified and recorded), the confined run, the signed verdict (appended to the ledger by the server) and a verified `audit_reveal`. It prints the fixed diagnostic, then arms the next nonce.
- **Trace:** `holden trace <run>` re-verifies and prints promise → approved cognition → root → lens → plan → nonce → params → verdict → ledger.
- **Limitation (personal mode):** the implementer and the agent share one OS user, so the role separation comes from which socket a caller reaches and from the file system, not from a second uid. The cryptographic seal (G-UNPREDICTABLE) does not depend on it.

## 4. Existing holdout assets used along the way

| Asset | Used for |
|---|---|
| S-CANON-MCP (`get_schema`, `get_doctrine_layer`, `check_canon_lock`) | Canon pin and lock checks (slice 0) |
| S-DOCTRINE spec, taxonomy, encounter profiles, rater-pilot data | Canon records. The measured 66.7% terminal-code agreement is a baseline to beat. |
| S-LENS and the owner's plain holdout skill | Lens and cognition corpus, and a dogfood harness until this one replaces it |
| S-RUNNER (Linux) | Differential verdict cross-check through the compatibility emitter |
| E-REVIEW | Design constraints, never-again controls and the feature backlog |
| The charter tool, the common engineering standard, ATLAS-mini | Governance, the standard, and the claim-ledger format |

## 5. Slice plan

| Slice | Delivers | Key proofs |
|---|---|---|
| 0 | Skeleton, license, charter, canon pin, `holden canon verify`, VRF round trip | Builds on macOS and in the Tart VM; canon matches the hosted pin; params depend on the evaluated commit; proofs verify |
| 1 | `holden-seal`: AEAD, commitments, key-store trait | Builder cannot derive params from L2; tamper refused |
| 2 | `holden-mcp` roles | Implementer refused `reveal_params` on both OSes; nonce ordering enforced |
| 3 | `holden-cognition` and `holden-gen` on a synthetic fixture project | Full provenance chain; no lens-invented promise reaches a check |
| 4 | `holden-eval`: rung 0 and confinement backends | Differential match with S-RUNNER on the same fixture |
| 5 | `holden-ledger`, controls and telemetry | E-REVIEW P0 mutants caught; authority from bounds |
| 6 | `modules/holden-forge` and `forge export` | The exported forge builds standalone |
| 7 | `holden embed` and the just-in-time loop | End to end in a second project |

## 6. Owner decisions

| Id | Decision | Status |
|---|---|---|
| D-01 | Intent card | Confirmed 2026-11-04 |
| D-02 | Name and remote | Default adopted: `holden-mcp`, private, owner's account, created when needed |
| D-03 | S-RUNNER code | Default adopted: clean-room until it is licensed |
| D-04 | Linux proof host | Owner chose a Tart VM on this Mac (2026-11-04) |
| D-05 | Post-verdict parameter visibility | Default adopted: auditors and owner only |
| D-06 | Vendor the S-FORGE methodology text | Resolved 2026-11-04: S-FORGE is the owner's proprietary work and is vendored under this repository's license, PolyForm Noncommercial 1.0.0 (`charter/owner-decisions/2026-11-04-license-polyform-noncommercial.md`) |
| D-07 | License | Resolved 2026-11-04: PolyForm Noncommercial 1.0.0, licensor named in `NOTICE` (`charter/owner-decisions/2026-11-04-license-polyform-noncommercial.md`) |
| D-08 | S-DOCTRINE canon files: commit them, or load them from the private root with only the pin committed | Resolved 2026-11-04: committed under `canon/holden/`; a private-root copy remains usable through `--root` (`charter/owner-decisions/2026-11-04-canon-committed.md`) |
