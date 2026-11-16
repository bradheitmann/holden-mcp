# Evidence — slice-2 independent QA — attempt 2

## Frozen bytes and toolchain

- Frozen commit: `c4d998e5ab8315c5aec5a3ec82bd71d9d9454796`.
- Freeze manifest SHA-256: `e237747a2136b9cf4a7298b53e5ddb2bcabe0b5acd47c0e51a45d0bc9d2bd91b`.
- Freeze tree: `3dc3dff4e7121568d7f682d7958e9f7bb02fac74`; 155 entries.
- Recomputed with `charter freeze slice-2` in the clean scratch clone: exact head, tree, entry count, and digest matched.
- `cargo --version`: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`.
- Both gates used clean checkouts of the frozen commit. Host target directories were under `/tmp/holden-mcp-qa2b-target-gate`; VM target directories were under `/tmp/holden-mcp-qa2b-target-linux`.

## Host gate

`cargo xtask gate slice-2` passed on macOS/aarch64 at the frozen commit with `tree clean: true`. Format, Clippy, tests, and the host guard passed. Test summary: **71 passed, 0 failed, 0 ignored**. Guard checks passed for languages, absolute-home-path scan, private identifiers, license checks, cargo-deny, cargo-audit, and Charter safety.

## Linux gate

`HOLDEN_VM_ADDR=192.168.64.10 cargo xtask gate slice-2 --linux-vm` returned exit 0 on Linux/aarch64. The VM checkout was fresh at the frozen commit and clean. Test summary: **71 passed, 0 failed, 0 ignored**. The gate compared names and counts with the host and reported exact equality. The peer-credential tests `implementer_refused_reveal_params`, `evaluator_allowed_reveal_params`, and `unmapped_uid_brief_only` passed on Linux. The VM guard reported declared limitations because `cargo-deny`, `cargo-audit`, and `charter` were absent; the full host guard passed those checks. VM build directories were confirmed under the specified `/tmp` target prefix.

## Attempt-2 mutation

In the clean host clone, removed the evaluator-role check from `reveal_params`. The named test `implementer_refused_reveal_params` failed because an implementer received a successful reveal. Restored the check, confirmed `git diff --exit-code`, then reran the named test successfully in its own Cargo target directory. No failure response or secret-bearing diagnostic is retained here.

## Attempt-1 evidence and archive integrity

The attempt-1 receipt records the P1–P6 fail-without mutants and the verifier P2/P5 mutants as executed, including red result and green-after-revert descriptions. They are classified here as **executed (attempt 1)**. Hashes computed for the archived attempt-1 files:

- `receipt.json`: `9d3564ffb573478c54a0b6f61966e4cc765ffe6ea42c4fe58feae308e961da4a`
- `report.md`: `a47dcf3438ad654a4a1e8a72a484f0d4baee45b6da2321d4237273eef8fcc5c6`
- `evidence.md`: `603dac8d9687cb47dc7c76ae99580227d374e243822aec7855f0658c087cd10c`

The attempt-1 receipt does not record hashes for its `report.md` or `evidence.md`. Therefore, a comparison of these computed hashes against receipt-recorded archive hashes is unavailable. The reported attempt-1 checks are transparently referenced but cannot be cryptographically authenticated to those two archive files.

## Slice-1 prerequisite

At main commit `0023b366e35c89c943a21c0a226371cd5f2d6df4`:

- `charter/.dispatch/slice-1.json` records `state: accepted`.
- `charter/owner-decisions/2026-11-05-slice-1-third-qa-attempt.md` records the owner's approval of the final QA attempt after S5 escalation.
- `qa/slice-1/receipt.json` records verdict `accepted`.

The requested `charter receipt qa/slice-1/receipt.json --verify-tree <scratch-at-0023b36>` command was run and refused: the later main tree is a descendant of the receipt candidate and the difference touches `Cargo.lock` outside the recorded freeze. To independently validate the receipt, a scratch checkout was set to its recorded candidate `cc677f6f4febc4ffaffc378aac2aea51a796be89`; the receipt, freeze record, reopen attestation, and QA pin were supplied from `0023b36` to that scratch checkout. Validation then passed with the exact recorded head and manifest digest. No source main checkout was modified.

## Transport boundary

Source inspection of `crates/holden-mcp/tests/mcp.rs` confirms the integration tests construct an `rmcp` client, connect to the actual Unix listener with `UnixStream::connect`, and launch the server as a child for stdio. Both transport tests passed in the host and Linux gates. `ManualClock` controls the expiry window only; no mock replaces the MCP server, socket, stdio transport, or peer credentials.

## Charter check

`charter check --strict` exited 0 and printed `charter: ok`. Existing missing cost-row warnings remain for accepted slices 0, 1b, 1c, and 6.
