# Done list — slice-3 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-3 --dispatch`
Scope: G-SITUATED, and completion criteria 3, 6 and 7. The work has three parts:
- The typed canon doctrine: what a holdout is, its roots, and how the roots generate cognitions.
- `holden-cognition`: planning document → verbatim promises → candidate cognitions → owner approval records.
- `holden-gen`: promise × approved cognition × lens draw → sealed plan, with a "needs owner words" output.
Proofs:
1. P1 canon-doctrine: `holden-canon` exposes a typed `Doctrine`. It has these parts:
   - A definition of a holdout.
   - Philosophical `Root`s. Each root carries an id, a name, and a citation into a committed canon file: file, section heading, and the SHA-256 of the cited section bytes. The roots are hospitality/the Other, situated cognition, the regenerative loop, measured reliability, and structural independence.
   - `Constituency`: human, agent, seam, successor.
   - `CircumstanceAxis`: attention, time pressure, interruption, device, permission, prior knowledge, hostility, handoff. Each axis has typed levels.
   - Generator rules mapping each root to the constituencies and axes it yields.
   Every citation resolves to existing bytes in `canon/holden/`. `holden canon doctrine` prints the doctrine.
   - Reopen: fresh clone; run the tests and `holden canon doctrine`.
   - Fail-without mutant: alter one citation hash. The test `every_root_citation_resolves` must fail.
   - Test: `holden-canon::tests::{every_root_citation_resolves, every_root_generates_something}`
2. P2 promises-verbatim: `holden-cognition` extracts promises from a planning document. A promise is a quoted span with byte offsets and the SHA-256 of the source file. A promise whose quoted bytes do not match the source at those offsets is refused. Paraphrase is impossible by construction: the promise text is sliced from the source bytes.
   - Reopen: run the tests against the committed synthetic fixture project.
   - Fail-without mutant: allow whitespace-normalized matching. The test `promise_quote_must_match_source_bytes` must fail.
   - Test: `holden-cognition::tests::{promise_quote_must_match_source_bytes, promise_carries_source_hash_and_offsets}`
3. P3 cognition-candidates-and-approval: for each promise, the doctrine generators propose candidate `SituatedCognition`s. They are deterministic for the same inputs. Each id is a content hash, and each candidate cites the root(s) and axes that produced it. A cognition can host tests only with an `Approval` record: the owner's Ed25519 signature over the cognition id and the promise id, made with an owner key from the KeyStore. An unapproved, wrongly signed, or differently keyed approval is refused.
   - Reopen: run the tests.
   - Fail-without mutant: skip signature verification. The test `unapproved_cognition_cannot_host_test` must fail.
   - Test: `holden-cognition::tests::{candidates_are_deterministic, unapproved_cognition_cannot_host_test, wrong_owner_key_is_refused}`
4. P4 generator-sealed-plans: `holden-gen` turns (approved promise, approved cognition, lens draw) into a typed plan. The plan contains encounter steps written from inside the cognition, a deterministic check per step, an absence probe, a permission pair, and a targeted broken variant. It is then sealed via `holden-seal`. Every plan carries this provenance chain: promise id → cognition id → root ids → lens concept ids. Generation is deterministic given the inputs and the draw. Different draws yield different plans.
   - Reopen: run the tests. Generate plans for the fixture and verify every chain link resolves.
   - Fail-without mutant: omit the root ids from provenance. The test `every_plan_traces_promise_cognition_root` must fail.
   - Test: `holden-gen::tests::{every_plan_traces_promise_cognition_root, different_draws_different_plans, plan_is_sealed}`
5. P5 needs-owner-words: when a lens implies a property not covered by any approved promise, the generator emits a `NeedsOwnerWords` item. It never emits a check for that property. Lens concept data alone can never produce a check.
   - Reopen: run the test with a fixture lens that implies an uncovered property.
   - Fail-without mutant: let the generator emit a check for a lens-implied property. The test `lens_implied_promise_goes_to_needs_owner_words` must fail.
   - Test: `holden-gen::tests::lens_implied_promise_goes_to_needs_owner_words`
6. P6 lens-draw-sealed-and-keyed: lens concepts for a plan are chosen by a keyed draw. The draw is HMAC-SHA-256 under a sealed draw key over (promise id, cognition id, epoch), weighted by corpus weights. It runs on the sealed side, so the builder cannot predict which lens shapes a plan. The draw is replayable from its inputs for audit.
   - Reopen: run the tests.
   - Fail-without mutant: make the draw unkeyed (plain SHA-256). The test `lens_draw_requires_key` must fail.
   - Test: `holden-gen::tests::{lens_draw_requires_key, lens_draw_replays}`
7. P7 forge-pack-loader: `holden-cognition` reads a domain pack and its constitutions produced by `forge export` (slice 6) purely as files, through a typed loader. It has no crate dependency on the forge. It loads the committed holdout-engineering pack losslessly. Its domain overlay changes the candidate cognitions it generates: a different pack gives different candidates, deterministically.
   - Reopen: export the forge to /tmp, load it, and run the tests.
   - Fail-without mutant: make the loader ignore the pack's overlay fields. The test `pack_overlay_changes_candidates` must fail.
   - Test: `holden-cognition::tests::{loads_forge_pack, pack_overlay_changes_candidates}`
Base note: this branch builds ahead on the unaccepted slice-2 and slice-1 candidates. Slice-3 acceptance requires both.
Gates: the same set as slice-2 (fmt, clippy, guard, host gate, VM gate with HOLDEN_VM_ADDR, `charter check --strict`).
Out of scope: running plans against a product (slice-4); telemetry weights (slice-5); a real owner signature on a real project (slice-7).
Owner decision points: none. The owner signs real approvals in slice-7.
