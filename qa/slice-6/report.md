# Independent QA report — slice-6

**Verdict: accepted with a medium environment limitation.** The frozen bytes passed the host gate and every slice proof was reopened. Linux matched the host's 65 test names and pass/fail/ignored counts, but its aggregate gate returned red because cargo-deny, cargo-audit, and charter are not installed in the VM. All three checks passed on the host.

## Checklist

- [x] Freeze manifest regenerated from a clean scratch clone: 135 entries, matching SHA-256 `bfc39c79a59621ac53933c34ebf87e4ccd5307c9c992956b78eeec4f7bd4c90b` at `761be1b1849799f54a818b748408473d8057ee6e`.
- [x] Cargo 1.95.0 recorded; scratch-clone gates and their inner Cargo runs used separate target directories under `/tmp/holden-mcp-qa6-target-*`.
- [x] Host `cargo xtask gate slice-6`: green, 65/65.
- [x] Linux gate: 65/65, exact test arrays match host; gate is red only for absent VM guard tools, covered by host.
- [x] P1 isolation passes. Main-workspace-to-forge and forge-to-main path dependency mutants each made `forge_is_isolated` fail. The named `holden-cognition` crate does not exist at the frozen commit; `holden-canon` was used as the main-workspace crate, which exercises the same invariant because the isolation rule applies to every main-workspace crate.
- [x] P2 fresh standalone export reopened on macOS and Linux: offline tests pass 13/13; forge verification and platform SHA256SUMS checks pass.
- [x] P2 absolute path mutant refused with the filename. Private identifier mutant refused with the filename; this report identifies it only as denylist entry 6.
- [x] P3 pack validation, trace, determinism, and lossless round-trip pass; dropping `/pack_name` from derivation makes the trace test fail.
- [x] P4 changed methodology file without checksum update is refused by filename.
- [x] Verifier mutant: modifying NOTICE is refused as both unpinned text and checksum mismatch.
- [x] Export scans found no parent repository name, user-home path, scratch checkout path, or machine address.
- [x] Every mutant was reverted; forge verification and the final host gate are green.

## Finding

D1 (medium, obligation): the VM does not have cargo-deny, cargo-audit, or charter. The Linux aggregate gate therefore records `red` despite 65/65 tests and all slice-specific forge checks passing. The host gate passed the three checks. No critical or high code defect was found.

## Validation

Receipt validation passed: `charter: tree . matches the recorded freeze for slice-6 (charter.freeze/2 (git-commit-tree 761be1b1849799f54a818b748408473d8057ee6e), manifest bfc39c79a59621ac53933c34ebf87e4ccd5307c9c992956b78eeec4f7bd4c90b)`.
