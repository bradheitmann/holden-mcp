# Done list — slice-0 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-0 --dispatch` (charter/.dispatch/slice-0.json)
Proofs (each: name, what it guarantees, reopen step, fail-without mutant):
1. P1 builds-on-both: The workspace builds and its tests pass on macOS aarch64 and in the Tart Linux VM, with no Docker or Python on either path (G-PORTABLE).
   - Reopen: fresh `git clone` of the frozen commit on the host and in the VM, then `cargo xtask gate slice-0`. Compare test-name lists and pass counts with the recorded run.
   - Fail-without mutant: use a `#[cfg(target_os = "macos")]`-only symbol unconditionally in `holden-canon`. The Linux gate must go red.
   - Test: `cargo xtask gate slice-0` (host) and `cargo xtask gate slice-0 --linux-vm` (Tart)
2. P2 canon-verify: `holden canon verify --root <dir> --pin <pin>` recomputes SHA-256 for every pinned file. It refuses any differing, missing or extra byte, naming the file. When the private canon root is absent, it returns the typed limitation `canon_absent`, never a pass (G-SITUATED provenance root).
   - Reopen: re-run on a fresh clone against the committed synthetic canon fixture. Compare the printed digest.
   - Fail-without mutant: flip one byte in the synthetic fixture. The command must exit 1 naming the file.
   - Test: `holden-canon::tests::{verify_refuses_modified_byte, absent_root_is_limitation_not_pass}`
3. P3 pin-matches-hosted: `canon/CANON-PIN.txt` holds exactly the four hashes and the validator hash served by S-CANON-MCP `check_canon_lock` (`skill-v2.3.0 adapter-v0.2.0`), as recorded verbatim in `docs/evidence/canon-mcp-pin-2026-11-04.txt`.
   - Reopen: call `check_canon_lock` again and diff the result against the recorded file. Re-run the test.
   - Fail-without mutant: change one hex digit in `CANON-PIN.txt`. The test must fail.
   - Test: `holden-canon::tests::pin_matches_recorded_hosted_pin`
4. P4 lens-corpus-contract: The committed synthetic lens corpus validates against the typed corpus model. It has no timestamped ids, and every id is a content hash. The private S-LENS corpus manifest is committed as hashes only, and every declared delta (stereotyped cognition rewrite, timestamp-free ids, no executable script) is listed in `canon/LENS-DELTAS.md`.
   - Reopen: fresh clone; run the test; `holden canon verify` the private corpus where the root exists, otherwise expect `canon_absent`.
   - Fail-without mutant: give one synthetic concept a timestamp-derived id. The test must fail.
   - Test: `holden-canon::tests::lens_ids_are_content_hashes`
5. P5 vrf-roundtrip: `holden-seal` draws moment-of-test parameters as `VRF_sk(plan_commit ‖ evaluated_commit ‖ server_nonce ‖ builder_nonce)`. A changed evaluated commit changes the parameters. The proof verifies under the epoch public key. A tampered proof or output is refused. Fixed vectors are byte-identical on macOS and Linux (G-UNPREDICTABLE core).
   - Reopen: re-run the tests on the host and in the VM, and compare the fixed-vector outputs byte for byte.
   - Fail-without mutant: drop `evaluated_commit` from the VRF transcript. `params_depend_on_evaluated_commit` must fail.
   - Test: `holden-seal::tests::{params_depend_on_evaluated_commit, reveal_proof_verifies, tampered_proof_is_refused, fixed_vector}`
6. P6 guard: No `.py`, `.js`, `.mjs`, `.cjs`, Dockerfile, absolute home path or private source name exists in tracked files. Every crate declares `license = "MIT"`, and LICENSE exists. `cargo deny check` and `cargo audit` pass. The charter safety scan passes (standard §1, §3).
   - Reopen: re-run `cargo xtask guard` and `charter safety` on a fresh clone.
   - Fail-without mutant: add `tools/x.py`. The guard must exit 1 naming it.
   - Test: `cargo xtask guard`
Gates (exact commands):
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask guard`
- `cargo xtask gate slice-0`
- `cargo xtask gate slice-0 --linux-vm`
- `charter check --strict` (pinned charter tool)
Out of scope (recorded obligations, not fixed here):
- AEAD sealing, key stores, the MCP server and roles: slices 1–2.
- Cognition, generation, evaluation, ledger, forge and embed: slices 3–7.
- A CI provider (a hosted or self-hosted runner without containers): to be decided before slice 1 acceptance.
Owner decision points:
- D-08: commit the S-DOCTRINE canon files, or load them from the private root (default: private root). This does not block the slice.
- Pending: "some other things that need to happen" (owner, 2026-11-04).
