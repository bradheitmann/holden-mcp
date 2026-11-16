# Evidence — slice-1c independent QA

## Frozen identity and bytes

- Checkout HEAD and candidate: `9668c6897fea2d300b5f8de556f425ef92660395`.
- Freeze record: `charter/.freeze/slice-1c.json`; tree `6b640c5b5e9fdbd1c358ea95c8ce04481a0b424c`; manifest population SHA-256 `53c78aa47e6f63f7be8ee786a347677e82b983a14e42b401e349b2e2d188c31e`.
- Compared 97 entries, ordered by path: SHA-256, Git object id, mode, type and path all matched. The actual commit has 101 paths; four under `charter/.freeze/` are intentionally excluded by charter.freeze/2 to avoid self-reference. Recomputed population text hash matched the recorded manifest SHA-256.
- Required environment: `export PATH="$HOME/.cargo/bin:$PATH"`; `cargo --version` → `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`.
- Scratch clone: `/tmp/holden-mcp-qa1c`, detached at the frozen commit. It was clean before and after verification; all mutants were isolated there and reverted.

## Host and Linux reopen

- Final host command: `CARGO_TARGET_DIR=/tmp/holden-mcp-qa1c-target-host-xtask HOLDEN_TARGET_BASE=/tmp/holden-mcp-qa1c-target-host-final cargo xtask gate slice-1c --record /tmp/holden-mcp-qa1c-host-run-final.json`.
- Host record: `/tmp/holden-mcp-qa1c-host-run-final.json`; macOS/aarch64; fmt, Clippy, tests and guard all pass; 38 passed, 0 failed, 0 ignored; `green`.
- VM route: `tart ip holden-mcp-linux` failed with `utimes(2) failed: Operation not permitted`; used the provided SSH endpoint and key route to send a Git bundle, then cloned the frozen commit freshly on Linux/aarch64. No VM lifecycle or configuration changes were made.
- Linux command: `CARGO_TARGET_DIR=/tmp/holden-mcp-qa1c-target-linux-xtask HOLDEN_TARGET_BASE=/tmp/holden-mcp-qa1c-target-linux cargo xtask gate slice-1c --host-covered-tools --record <external-run-record>`.
- Retrieved Linux record: `/tmp/holden-mcp-qa1c-linux-run.json`. Result `green_with_declared_limitations`; 38 passed, 0 failed, 0 ignored. Missing `cargo-deny`, `cargo-audit` and charter were explicitly reported as limitations and passed on host.
- Compared both JSON records programmatically: `tests.names` equal; passed, failed and ignored counts equal. Test names:

```text
holden-canon::tests::absent_root_is_limitation_not_pass [ok]
holden-canon::tests::committed_canon_matches_pin [ok]
holden-canon::tests::committed_synthetic_canon_fixture_verifies [ok]
holden-canon::tests::import_refuses_non_empty_destination [ok]
holden-canon::tests::import_reproduces_committed_lens_fixture [ok]
holden-canon::tests::lens_fixture_matches_its_manifest [ok]
holden-canon::tests::lens_ids_are_content_hashes [ok]
holden-canon::tests::pin_matches_recorded_hosted_pin [ok]
holden-canon::tests::pin_parse_refuses_escaping_paths [ok]
holden-canon::tests::pin_parse_refuses_uppercase_or_short_digest [ok]
holden-canon::tests::verify_refuses_extra_file [ok]
holden-canon::tests::verify_refuses_missing_file [ok]
holden-canon::tests::verify_refuses_modified_byte [ok]
holden-cli::canon_verify_absent_root_exits_3_canon_absent [ok]
holden-cli::canon_verify_defaults_to_committed_canon [ok]
holden-cli::canon_verify_exits_1_naming_modified_file [ok]
holden-cli::canon_verify_passes_committed_synthetic_fixture [ok]
holden-cli::import_lens_reproduces_committed_fixture_manifest [ok]
holden-seal::tests::evaluated_commit_must_be_lowercase_hex_of_git_length [ok]
holden-seal::tests::fixed_vector [ok]
holden-seal::tests::other_epoch_key_is_refused [ok]
holden-seal::tests::params_depend_on_evaluated_commit [ok]
holden-seal::tests::reveal_for_other_inputs_is_refused [ok]
holden-seal::tests::reveal_proof_verifies [ok]
holden-seal::tests::tampered_output_is_refused [ok]
holden-seal::tests::tampered_params_are_refused [ok]
holden-seal::tests::tampered_proof_is_refused [ok]
xtask::gate::tests::parses_libtest_result_line [ok]
xtask::guard::tests::languages_refuses_python_and_js_and_dockerfiles [ok]
xtask::guard::tests::stale_license_mentions_ignore_only_third_party_and_records [ok]
xtask::guard::tests::undeclared_limitation_is_not_a_pass [ok]
xtask::guard::tests::whole_word_matching [ok]
xtask::tests::license_text_hash_is_pinned [ok]
xtask::tests::licensor_exception_lists_every_file [ok]
xtask::tests::new_decision_claiming_mit_is_refused [ok]
xtask::tests::qa_record_exempt_only_when_pinned [ok]
xtask::tests::unpinned_qa_file_is_checked [ok]
xtask::util::tests::utc_formats_known_instants [ok]
```

## Proof mutants and reverts

1. **P1 unpinned file:** staged `qa/slice-x/notes.md` with `This repository is MIT licensed`. `cargo xtask guard` exited 1 and named `qa/slice-x/notes.md:1`. Removed file; subsequent guard passed.
2. **P1 changed pin bytes:** changed one byte in `qa/slice-1b/evidence.md`. `cargo xtask guard` exited 1 and refused that file because bytes no longer matched the pin. Restored original bytes; guard passed and named its accepted-slice SHA-256 pin reason.
3. **P2 restored directory exemption:** temporarily added `charter/owner-decisions/` to `STALE_LICENSE_EXEMPT`. `cargo test --locked -p xtask new_decision_claiming_mit_is_refused` failed: expected refusal, observed `Pass`. Restored `xtask/src/guard.rs`; test passed.
4. **P2 new decision probe:** staged a new owner-decision file stating `This repository is MIT licensed`. Guard exited 1 and named the probe at line 1. Removed the file; guard passed.
5. **P3 omitted exception:** removed the `charter/DONE/slice-1b.md` entry from the licensor exception record. `cargo test --locked -p xtask licensor_exception_lists_every_file` failed and named the omitted carrier. Restored the record; test passed.
6. **Verifier mutant:** changed `slice_accepted` to accept any parseable dispatch record regardless of state. `cargo test --locked -p xtask qa_record_exempt_only_when_pinned` failed because the non-accepted slice was exempted. Restored `xtask/src/guard.rs`; test passed.

After all reverts: P1 `qa_record_exempt_only_when_pinned`, P2 `new_decision_claiming_mit_is_refused`, P3 `licensor_exception_lists_every_file`, and `cargo xtask guard` all passed. The complete host gate was rerun green.

## Charter

`charter check --strict` → `charter: ok`, with existing cost-row warnings for accepted slice-0 and slice-1b. Receipt verification command: `charter receipt qa/slice-1c/receipt.json --verify-tree .`.
