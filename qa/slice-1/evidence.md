# slice-1 QA evidence — attempt 3

## Frozen bytes and environment

- Candidate and frozen head: `cc677f6f4febc4ffaffc378aac2aea51a796be89`.
- Tree: `63c91885445d7f57d8ef442db79f87f45fc649a3`.
- Manifest: 81/81 entries matched by path, object id, mode, type, and SHA-256; population digest `43454d6d223a8637856121b193ca932d30e5f5ea9b43e3e5c1ee218dbdfe5037`.
- Patch SHA-256: `null` in the freeze record.
- Cargo: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`.

## Gates

| Gate | Result | Evidence |
|---|---|---|
| Host gate, clean scratch clone | Green | macOS/aarch64; frozen HEAD; fmt, Clippy, all 48 tests, guard, cargo-deny, cargo-audit, Charter safety passed. |
| Linux VM gate, clean clone | Green with declared limitations | Linux/aarch64; all 48 test names and outcomes identical to host. VM lacks cargo-deny, cargo-audit, and Charter; these ran successfully on host. Target directories stayed under the requested target base. |
| Strict Charter check | Green | `charter: ok`; one existing slice-0 no-cost-row warning. |
| Post-mutant full workspace tests | Green | 48 passed, 0 failed, 0 ignored on the frozen scratch clone. |

Both host and Linux tests passed `seal_fixed_vector`, asserting the same commit bytes: `2d368c47d0cdb02515b2225e1509a885844309a504f6c2109b5e525f014b8bb5`.

## Mutants

| Mutant | Test or gate | Observed result |
|---|---|---|
| M1: omit AAD from `plan_commit` | `commit_binds_aad` | Red; fixed-vector commitment bytes differed. |
| M2: decrypt ciphertext with raw XChaCha20 and discard the authentication tag | `tampered_ciphertext_is_refused` | Red; unauthenticated tampering reached `PlanMalformed` instead of the required tag refusal. |
| M3: derive parameters from SHA-256 of plan and evaluated commits | `params_require_server_secret` | Red; parameters no longer differed between server keys. |
| M4: let `open()` accept every nonce | `uncommitted_nonce_is_refused` | Red; returned `Ok(())`. |
| M5: disable mode validation while loading file keys | `world_readable_key_is_refused` | Red; world-readable key loaded. |
| M6: ignore `HOLDEN_VM_ADDR` | `vm_addr_override_skips_tart` | Red; resolver attempted absent `tart`. |
| M6 end-to-end: same override mutant with `tart` absent from `PATH` | `cargo xtask gate slice-1 --linux-vm` | Exit 1 and refused because `tart ip` provided no address. |
| Independent P3 mutant: omit evaluated commit from VRF transcript | `params_depend_on_evaluated_commit` | Red; the evaluated commit no longer influenced the draw. |

Every mutant ran in a disposable scratch clone, and none changed the frozen candidate. The clean candidate test suite passed after the mutations were discarded.
