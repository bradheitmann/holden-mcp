# Done list — slice-1b (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-1b --dispatch` (charter/.dispatch/slice-1b.json)
Scope: relicense the repository from MIT to PolyForm Noncommercial 1.0.0 (owner decision D-07, 2026-11-04) and commit the S-DOCTRINE canon files (owner decision D-08, 2026-11-04). No feature work.
Proofs (each: name, what it guarantees, reopen step, fail-without mutant):
1. P1 license-text-is-official: `LICENSE` is byte-identical to `PolyForm-Noncommercial-1.0.0.md` from the PolyForm project at upstream commit `76a278c402bc43b8d2b561da140b0f3e17263015` (sha256 `c0ea4a896d2c8c394b29f9427589996db826cd501c512279ff0ed3ef48fabbe5`). The Required Notice lines live in `NOTICE`: "Required Notice: Copyright (c) 2026 Bradley Heitmann" and "Required Notice: Commercial use requires a separate license from the licensor; contact the licensor through the repository's issue tracker."
   - Reopen: fetch the upstream file at the pinned commit and compare its sha256 with `LICENSE`. Re-run the guard.
   - Fail-without mutant: change one character of `LICENSE`. The guard must exit 1 naming the license-text check.
   - Test: `cargo xtask guard` (check `license_text`) and `xtask::tests::license_text_hash_is_pinned`
2. P2 every-crate-polyform: every workspace crate, including the forge workspace when it exists, declares `license = "PolyForm-Noncommercial-1.0.0"`. The guard refuses any crate declaring another license and names it. `cargo deny` allows this license for workspace crates only, and all third-party licenses are unchanged.
   - Reopen: re-run `cargo xtask guard` and `cargo deny check` on a fresh clone.
   - Fail-without mutant: set one crate to `license = "MIT"`. The guard must exit 1 naming that crate.
   - Test: `cargo xtask guard` (check `crate_licenses`)
3. P3 canon-committed-and-verified: the four S-DOCTRINE canon files are committed under `canon/holden/`, byte-identical to the hosted pin. `holden canon verify` defaults to the committed root and passes. The private-root path remains available as an override. Private names, absolute paths and secrets are scanned in these files and none are present.
   - Reopen: on a fresh clone, run `holden canon verify` and compare the digest with `canon/CANON-PIN.txt`. Re-run the guard.
   - Fail-without mutant: flip one byte in `canon/holden/holdout_skill_spec_v2_3_0.md`. Verify must exit 1 naming the file.
   - Test: `holden-canon::tests::committed_canon_matches_pin`
4. P4 docs-consistent: no tracked file still states the repository is MIT-licensed (README, ARCHITECTURE, STANDARD-ADOPTION, AGENTS). Owner decisions D-06, D-07 and D-08 are recorded in `charter/owner-decisions/`. The licensor-name exception to the common standard §1 is recorded there too, with its reason, owner and expiry (none).
   - Reopen: `git grep -n -i 'MIT'` shows only third-party-license contexts. Run `charter check --strict`.
   - Fail-without mutant: restore "MIT. See LICENSE" in README. The guard's `stale_license_mentions` check must fail.
   - Test: `cargo xtask guard` (check `stale_license_mentions`)
Gates (exact commands):
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask guard`
- `cargo xtask gate slice-1b`
- `HOLDEN_VM_ADDR=<vm address> cargo xtask gate slice-1b --linux-vm`
- `charter check --strict`
Out of scope (recorded obligations, not fixed here):
- The forge module itself (slice 6). It inherits this license.
- Any history rewrite of earlier MIT-licensed commits. They were never published (the repository is private), and a rewrite would be an owner decision.
Owner decision points:
- None remaining. The licensor is "Bradley Heitmann" (the owner's chosen name in the 2026-11-04 instruction). The commercial contact is the repository issue tracker; the owner may replace it with an email address later.
