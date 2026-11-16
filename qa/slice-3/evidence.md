# Slice 3 QA attempt 2 evidence

Frozen candidate: `99a5bbfe14c935f470acec8bd6adac78f058bc20`.
Manifest: `842b97327d8a7222332f46ff7943c50efdbfe29af224e96f1af3343572a43601`.
Toolchain: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`.

## Prerequisites

From the main holden-mcp checkout, the charter tool validated both prerequisite receipt trees against independent scratch clones under `/tmp`:

- Slice 1 at `cc677f6f4febc4ffaffc378aac2aea51a796be89`: `tree matches the recorded freeze`, exit 0.
- Slice 2 at `c4d998e5ab8315c5aec5a3ec82bd71d9d9454796`: `tree matches the recorded freeze`, exit 0.

The main checkout dispatch records report both slices accepted.

## Mutants

Each mutant ran in its own scratch clone of the frozen commit. Each red and green `cargo test` invocation used a unique `CARGO_TARGET_DIR` under `/tmp/holden-mcp-qa3-target-*`. After red, the mutation was reverted before the green invocation.

| Mutant | Mutation and command | Red result | Reverted result |
|---|---|---|---|
| M1 | Changed one doctrine citation hash; `cargo test --locked -p holden-canon every_root_citation_resolves` | `tests::every_root_citation_resolves ... FAILED`; `HashDiffers` | `... ok`, 1 passed |
| M2 | Changed `quote_matches` to accept normalized/altered bytes; `cargo test --locked -p holden-cognition promise_quote_must_match_source_bytes` | `tests::promise_quote_must_match_source_bytes ... FAILED`; trailing-space quote was accepted | `... ok`, 1 passed |
| M3 | Skipped signature verification; `cargo test --locked -p holden-cognition unapproved_cognition_cannot_host_test` | exit 101, test failed | `... ok`, 1 passed |
| M4 | Replaced provenance root ids with an empty vector; `cargo test --locked -p holden-gen every_plan_traces_promise_cognition_root` | exit 101, test failed | `... ok`, 1 passed |
| M5 | Cleared `NeedsOwnerWords`; `cargo test --locked -p holden-gen lens_implied_promise_goes_to_needs_owner_words` | exit 101, test failed | `... ok`, 1 passed |
| M6 | Replaced HMAC draw output with plain SHA-256; `cargo test --locked -p holden-gen lens_draw_requires_key` | exit 101, test failed | `... ok`, 1 passed |
| M7 | Ignored overlay by replacing it with `None`; `cargo test --locked -p holden-cognition pack_overlay_changes_candidates` | `tests::pack_overlay_changes_candidates ... FAILED` | `... ok`, 1 passed |

## Attempt 1 evidence carried forward

The attempt 1 archive was checked at `$AGENT_PRIVATE_ROOT/holden-mcp/evidence/qa/slice-3-attempt1/` and contains receipt, report and evidence. Its host gate, Linux gate, P1 citation review, P2/P3/P5 inspections, and independent G-SITUATED mutant are recorded as executed/inspected in the new receipt. The Linux VM guard limitation is informational: the VM lacks cargo-deny, cargo-audit and charter, while the host guard covered these tools; prior accepted slices declare the same limitation.

## P1 citation review

The committed doctrine roots and canon sections reviewed in attempt 1:

- Hospitality and the Other: `The Friendship Principle` grounds ethical accommodation before classification and the obligation to an unassumed participant.
- Situated cognition: `Situatedness and Product-Class Configuration` and `Encounter Profile Multiplexing` support circumstance-specific encounters.
- Regenerative loop: `The Learning Loop` grounds feedback inputs and update cadence. The source does not use the word “regenerative”; that label is interpretive.
- Measured reliability: `The Self-Doubt Commitment` and self-evaluation mechanisms support uncertainty, controls and earned authority.
- Structural independence: `The Testing Paradox` and the air-gap section support separation between builder visibility and test reasoning.

## P2, P3, P5 inspection

- P2: `Promise::slice` gets text from the exact byte span; quote comparison uses byte equality; verification rechecks offsets, source hash and bytes.
- P3: `HostableCognition::admit` rejects missing approvals, mismatched IDs, a different trusted owner key, malformed signatures and Ed25519 verification failures. Admission is the construction path for the hostable wrapper.
- P5: an uncovered lens implication adds `NeedsOwnerWords`; generated checks remain based on approved promises.

## Receipt validation

Run from this checkout: `~/open_protocols/charter/target/release/charter receipt qa/slice-3/receipt.json --verify-tree .`.
