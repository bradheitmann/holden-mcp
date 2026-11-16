# Done list — slice-2 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-2 --dispatch` (charter/.dispatch/slice-2.json)
Base note: this branch starts from the unaccepted slice-1 candidate (cc677f6, held under S5) and must merge `main` (slices 1b, 1c, 6) first. Slice-2 acceptance requires slice-1 acceptance.
Run close (amended before any slice-2 code, 2026-11-05): `begin_run` records a run window from the sealed server config. When the window expires the run is closed: `reveal_params` is refused with `run_closed`, and `audit_reveal` is allowed. The slice-4 `verdict` tool will close runs early.
Scope: the `holden-mcp` crate. It is an MCP server (official Rust MCP SDK `rmcp`, exact pin) over stdio and a Unix socket. It exposes `brief`, `commit_nonce`, `begin_run`, `reveal_params` and `audit_reveal` on top of `holden-seal`, with roles authorized by socket peer credentials. This is G-UNPREDICTABLE at the service boundary (docs/ARCHITECTURE.md §3.2).
Proofs (each: name, what it guarantees, reopen step, fail-without mutant):
1. P1 mcp-surface: a real MCP client (the `rmcp` client in tests, over a real Unix socket and over stdio) lists exactly the five tools, each with a JSON schema, and completes an end-to-end run:
   1. `commit_nonce`
   2. `begin_run(plan_commit, evaluated_commit, builder_nonce)`
   3. `reveal_params` (evaluator)
   4. close
   5. `audit_reveal`
   The revealed parameters verify with `holden-seal::verify` under the published epoch public key.
   - Reopen: fresh clone; run the integration test on the host and in the VM.
   - Fail-without mutant: drop `audit_reveal` from the tool router. The test `lists_exactly_five_tools` must fail.
   - Test: `holden-mcp::tests::{lists_exactly_five_tools, end_to_end_run_over_socket, end_to_end_run_over_stdio}`
2. P2 role-by-peer-credentials: on the Unix socket, the caller's uid is read from the kernel via tokio's `UnixStream::peer_cred()` (`getpeereid` on macOS, `SO_PEERCRED` on Linux inside tokio; no `unsafe` in this repository) and mapped to a role by the sealed server config. An `implementer` caller gets a typed `forbidden_role` refusal from `reveal_params`, and the response leaks no parameter, proof or sealed byte. An `evaluator` caller succeeds. An unmapped uid is refused for everything except `brief`. Stdio mode is evaluator-only and must be explicitly enabled in config.
   - Reopen: run the tests on both OSes. Each test maps the current uid to each role in turn through config, and reads the real peer credentials.
   - Fail-without mutant: skip the role check in `reveal_params`. The test `implementer_refused_reveal_params` must fail.
   - Test: `holden-mcp::tests::{implementer_refused_reveal_params, evaluator_allowed_reveal_params, unmapped_uid_brief_only}`
3. P3 nonce-ordering-and-replay: `begin_run` is refused unless a server-nonce commitment for that plan was recorded first. Each server nonce opens at most one run (a replay is refused). Every run record stores the commitment, both nonces, the evaluated commit and the time of each step. A `reveal_params` call for a run that was never begun is refused.
   - Reopen: run the tests.
   - Fail-without mutant: allow a second `begin_run` with the same server nonce. The test `server_nonce_replay_is_refused` must fail.
   - Test: `holden-mcp::tests::{begin_without_commitment_is_refused, server_nonce_replay_is_refused, reveal_without_run_is_refused}`
4. P4 audit-after-close-only: `audit_reveal` refuses an open run with `run_open`. After close it returns the parameters plus the VRF proof to any role, and they verify. Before close, no role other than evaluator can obtain the parameters by any tool.
   - Reopen: run the tests.
   - Fail-without mutant: let `audit_reveal` answer open runs. The test `audit_refuses_open_run` must fail.
   - Test: `holden-mcp::tests::{audit_refuses_open_run, audit_after_close_verifies}`
5. P5 no-secret-egress: no response, error message or log line, for any role, contains the VRF secret key, the AEAD key, sealed-plan plaintext, or (before close) parameters. A test drives every tool with every role through success and failure paths. It scans all response bytes and captured logs for known secret byte patterns from a seeded run.
   - Reopen: run the test.
   - Fail-without mutant: include the run's parameters in the `forbidden_role` error text. The test `no_secret_in_any_response_or_log` must fail.
   - Test: `holden-mcp::tests::no_secret_in_any_response_or_log`
6. P6 local-bind-only: the optional TCP transport binds only to `127.0.0.1` or `::1`, on a configured port. A config naming `0.0.0.0`, `::` or a non-loopback address is refused at startup with a typed error (standard §2). The default transport is the Unix socket with mode 0600, in a 0700 directory.
   - Reopen: run the tests.
   - Fail-without mutant: accept `0.0.0.0`. The test `non_loopback_bind_is_refused` must fail.
   - Test: `holden-mcp::tests::{non_loopback_bind_is_refused, socket_mode_is_0600}`
Gates (exact commands):
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask guard`
- `cargo xtask gate slice-2`
- `HOLDEN_VM_ADDR=<vm address> cargo xtask gate slice-2 --linux-vm`
- `charter check --strict`
Out of scope (recorded obligations, not fixed here):
- `submit_observation`, `verdict` and `telemetry` (slices 4–5).
- Remote role tokens (later).
- Separate-OS-user deployment proofs (they need a second user account; a later slice).
- Keychain and keyring key stores.
Owner decision points:
- None.
