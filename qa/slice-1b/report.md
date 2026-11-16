# Slice 1b independent QA report

**Verdict: accepted with one medium obligation.** All four proofs reopened on the frozen bytes. The required mutants failed as specified, the clean tree passed host and Linux gates, and the independently fetched license matched exactly. A separate P4 probe found that the guard exempts every file under `charter/owner-decisions/`; a current contrary license claim in one of those files passed the guard. The frozen files contain historical decision context there, not a current claim, so this is a guard coverage defect recorded as D1 rather than a present-state contradiction.

## Proof results

- **P1 — reopened.** All 84 frozen file hashes matched. The pinned upstream PolyForm license was fetched with the requested `gh api` command. Both upstream bytes and `LICENSE` hash to `c0ea4a896d2c8c394b29f9427589996db826cd501c512279ff0ed3ef48fabbe5`; `cmp` matched. A one-byte LICENSE mutation made `license_text` refuse with exit 1; restoring it returned the guard to Pass.
- **P2 — reopened.** The clean host gate passed `crate_licenses`. Changing `holden-cli` to `license = "MIT"` made the guard exit 1 and name the crate. Restoring the manifest returned the guard to Pass. Host `cargo deny check` and `cargo audit` passed.
- **P3 — reopened.** Fresh-clone `holden canon verify` passed four files with digest `c50c0170b89590e007c4fff12281faab63a552cdc66a00e488998da3c4ab9cb6`, matching `canon/CANON-PIN.txt`. Flipping one byte in the specified canon file made verification exit 1 and name it; restoring it returned verification to green.
- **P4 — reopened.** Restoring `MIT. See LICENSE` in README made `stale_license_mentions` fail and named `README.md:32`; restoring README returned the guard to Pass. `charter check --strict` returned `charter: ok` with an existing slice-0 missing-cost-rows warning. A separate probe showed a current false license claim in an owner-decision file is exempt and passes the guard; D1 records this limitation.

## Gate results

The scratch-clone host gate passed on macOS/aarch64 with 34/34 tests, format, Clippy, all guard checks, cargo-deny, cargo-audit, and charter safety. The Linux Tart entrypoint refused because `tart ip holden-mcp-linux` returned no address and this frozen driver does not consume `HOLDEN_VM_ADDR`. I shipped the exact frozen commit as a Git bundle to the provided VM over SSH and directly ran its Linux gate on Linux/aarch64. It returned `green_with_declared_limitations`: 34/34 tests passed; cargo-deny, cargo-audit, and charter were absent and host-covered. Retrieved host and VM records had identical test names and counts (34 passed, 0 failed).

The first exploratory host gate used the xtask default build directories outside the required `holden-mcp-qa1b-target-*` prefix. I reran the gate with `HOLDEN_TARGET_BASE` and `CARGO_TARGET_DIR` under that required prefix; the compliant rerun was green, and all subsequent Cargo runs used distinct target directories under the required prefix.

## P4 reopen wording

The literal command `git grep -n -i 'MIT'` performs substring matching and therefore includes incidental matches such as “commit” and “limit”; it cannot literally produce only third-party-license contexts. Reading the intended review as a whole-word search with historical decision/DONE records and third-party allow-list entries understood in context is reasonable for the current tree. It does not fully enforce the promise across future tracked text: the blanket owner-decision directory exemption allowed a current “repository is MIT-licensed” statement in the scope probe. See D1.

## D1 — medium obligation

`stale_license_mentions` exempts all of `charter/owner-decisions/`, not only lines that document the former license or quote the owner’s historical decision. A present-tense contradictory license claim in that directory bypasses the guard. Narrow the exemption to historical contexts or otherwise ensure current contradictory claims are checked.
