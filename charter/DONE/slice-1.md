# Done list — slice-1 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-1 --dispatch` (charter/.dispatch/slice-1.json)
Scope: L2 sealing and the commitment primitives behind G-UNPREDICTABLE (docs/ARCHITECTURE.md §3.2), a file key store, and the two xtask obligations from slice-0. The MCP server and its roles are slice 2.
Proofs (each: name, what it guarantees, reopen step, fail-without mutant):
1. P1 seal-roundtrip: `holden-seal` seals a typed plan with XChaCha20-Poly1305 under a 32-byte sealed key. Associated data binds the plan id and format version. `plan_commit = SHA-256(ciphertext ‖ nonce ‖ aad)` is computed from the sealed bytes only. Unsealing with the right key returns the identical typed plan.
   - Reopen: fresh clone, run the tests on the host and in the VM, and compare the fixed-vector commit bytes across both OSes.
   - Fail-without mutant: drop the aad from `plan_commit`. The test `commit_binds_aad` must fail.
   - Test: `holden-seal::tests::{seal_roundtrip, commit_binds_aad, seal_fixed_vector}`
2. P2 tamper-refused: any change to the ciphertext, nonce or aad, or a wrong key, is refused with a typed error naming which check failed. Unseal never returns partial plaintext.
   - Reopen: re-run the tests.
   - Fail-without mutant: skip the AEAD tag check by decrypting without verification. The test `tampered_ciphertext_is_refused` must fail.
   - Test: `holden-seal::tests::{tampered_ciphertext_is_refused, tampered_aad_is_refused, wrong_key_is_refused}`
3. P3 l2-alone-yields-no-params: the moment-of-test parameters cannot be computed from L2 (sealed plan plus `plan_commit`) without the server's VRF secret key. The draw API takes the secret key as a distinct type that L2 artifacts cannot produce. For the same L2 and the same evaluated commit, two different server keys produce different parameters, and only the matching public key verifies each.
   - Reopen: re-run the tests.
   - Fail-without mutant: derive the parameters from a hash of `plan_commit ‖ evaluated_commit` instead of the VRF output. The test `params_require_server_secret` must fail.
   - Test: `holden-seal::tests::{params_require_server_secret, foreign_key_proof_is_refused}`
4. P4 nonce-commitment: `NonceCommitment = SHA-256(domain ‖ server_nonce)` is published before the builder's commit exists. `open(commitment, nonce)` accepts only the committed nonce. A draw whose `server_nonce` does not open the recorded commitment is refused before any parameters are produced.
   - Reopen: re-run the tests.
   - Fail-without mutant: make `open` accept any nonce. The test `uncommitted_nonce_is_refused` must fail.
   - Test: `holden-seal::tests::{nonce_commitment_opens, uncommitted_nonce_is_refused}`
5. P5 file-key-store: the `KeyStore` trait has a file backend that creates key files with mode 0600, owned by the current user, inside a 0700 directory. It refuses to load a key whose file or directory is group- or world-accessible, or owned by another user, naming the path and the mode. The same behaviour holds on macOS and Linux. Keys are never printed or logged: `Debug` and `Display` redact them.
   - Reopen: re-run the tests on the host and in the VM.
   - Fail-without mutant: drop the mode check on load. The test `world_readable_key_is_refused` must fail.
   - Test: `holden-seal::tests::{file_store_roundtrip, world_readable_key_is_refused, key_debug_is_redacted}`
6. P6 vm-gate-addr-and-target: `cargo xtask gate <slice> --linux-vm` honours `HOLDEN_VM_ADDR`, using that address without calling `tart`. It forwards the per-run target-directory base, so the VM run record shows a target directory under that base. A sandboxed verifier can therefore run the Linux gate.
   - Reopen: run the VM gate with `HOLDEN_VM_ADDR` set and `tart` absent from `PATH`, then inspect the returned run record.
   - Fail-without mutant: ignore `HOLDEN_VM_ADDR`. The gate run with `tart` absent from `PATH` must fail.
   - Test: `xtask::tests::vm_addr_override_skips_tart` and the reopen step above
Gates (exact commands):
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask guard`
- `cargo xtask gate slice-1`
- `HOLDEN_VM_ADDR=<vm address> cargo xtask gate slice-1 --linux-vm`
- `charter check --strict`
Out of scope (recorded obligations, not fixed here):
- macOS Keychain and Linux kernel keyring backends: a later slice, after the file backend is proven.
- The MCP server, roles and peer credentials: slice 2.
- Token input/output split capture for the cost ledger: process obligation, recorded in STATE.
Owner decision points:
- None inside the slice. D-08 and D-06 remain open.
