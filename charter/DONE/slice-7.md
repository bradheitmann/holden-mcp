# Done list — slice-7 (written BEFORE dispatch; does not grow during it)
Base bytes: the commit recorded by `charter gate slice-7 --dispatch`
Scope: completion criterion 12, plus the end-to-end proof of G-SITUATED, G-UNPREDICTABLE and G-MEASURED in a real second project. `holden embed <project>` installs a separate holden agent into the host project. A post-commit hook there triggers a just-in-time sealed holdout run end to end.
Proofs:
1. P1 embed-installs: `holden embed <project>` writes these into a real host git repository:
   - `.holden/config.json` (the public run file);
   - `.holden/agents/holden.md` (a constitution forged for the host's domain via the forge's export, not imported from it);
   - `.holden/cognitions/` (public halves only);
   - an MCP client config pointing at the holden server's socket with the implementer role;
   - a `post-commit` hook.

   Nothing sealed lands in the host repository: no plan plaintext, no keys and no parameters. Re-running embed is idempotent.
   - Reopen: on both OSes, embed into a fresh clone of the host and scan the host tree for sealed bytes and keys.
   - Fail-without mutant: write the sealed plan into `.holden/`. The test `no_sealed_bytes_in_host` must fail.
   - Test: `holden-embed::tests::{installs_expected_files, no_sealed_bytes_in_host, embed_is_idempotent}`
2. P2 separate-agent-identity: the embedded holden agent runs as its own process with its own constitution and the evaluator role, through the server's role map. The host's implementer-role MCP config cannot call `reveal_params`; it gets `forbidden_role`. It can call `brief`, which returns only public halves.
   - Reopen: from the host checkout, call the server with the installed implementer config.
   - Fail-without mutant: install the implementer config with the evaluator role. The test `host_implementer_cannot_reveal` must fail.
   - Test: `holden-embed::tests::host_implementer_cannot_reveal`
3. P3 just-in-time-end-to-end: in the host repository, `git commit` fires the post-commit hook, which runs this sequence:
   1. The evaluator role calls `commit_nonce`, using a nonce committed before this commit existed.
   2. It calls `begin_run` with the new commit's SHA.
   3. `reveal_params` returns the moment-of-test parameters.
   4. The holdout runs in confinement.
   5. A signed verdict is recorded in the ledger.
   6. The host gets the fixed diagnostic.

   Two consecutive commits get different parameters. After the run, `audit_reveal` verifies. The whole chain (promise → approved cognition → root → lens → plan → params → verdict) is printed by `holden trace <run>`.
   - Reopen: on both OSes, make two real commits in a fresh clone of the host and run `holden trace` and `holden replay` on both runs.
   - Fail-without mutant: have the hook reuse the previous run's parameters. The test `consecutive_commits_get_different_params` must fail.
   - Test: `holden-embed::tests::{post_commit_runs_jit_holdout, consecutive_commits_get_different_params}`, plus the reopen step
4. P4 owner-approved-in-host: the host's promises and cognitions are approved by the owner's real signing key, held in the owner's KeyStore and never by an agent. Runs on unapproved items are refused. The approval step is a single `holden approve` command for the owner, with its output recorded.
   - Reopen: verify the approval signatures against the owner's public key.
   - Fail-without mutant: accept an approval signed by an agent key. The test `agent_signed_approval_is_refused` must fail.
   - Test: `holden-cognition::tests::agent_signed_approval_is_refused`
5. P5 read-confinement-enables-block: product processes cannot read the private root, any key store, or the sealed side.
   - On macOS, the `sandbox-exec` profile denies `file-read*` there.
   - On Linux, the Landlock ruleset grants read access only to the world directory and the system paths needed to run, so other reads are denied.
   - A fixture variant that tries to read a planted sealed file fails to.
   - With reads confined, the `file_reads_not_confined` limitation is dropped, and a check whose measured bounds meet the block bar reaches `block` in the ledger.
   - Reopen: run the tests on the host (and in the VM when available).
   - Fail-without mutant: remove the read restriction. Both tests `sealed_read_is_denied` and `block_reachable_when_bounds_met` must fail.
   - Test: `holden-eval::tests::sealed_read_is_denied` and `holden-ledger::tests::block_reachable_when_bounds_met`
Base note: this branch builds ahead on the unaccepted slice-1 to slice-5 candidates. Acceptance requires those first. Recorded orchestrator decisions from slice-5:
   - Linux confinement violations are unobservable, so they carry a typed limitation capped at `recommend`.
   - The recommend bar is catch lower bound ≥ 0.50 and false-fail upper bound ≤ 0.25, with minimum samples of 10 known-broken and 10 known-good runs.
   - The ledger key lives in the private-root file key store.
Gates: same as slice-2, plus the end-to-end reopen on both OSes.
Host project: a small real git project (strongly typed, MIT or owned by the owner) created as a fixture repository under the private root, so no private project is touched.
Out of scope: embedding into the owner's other live repositories.
Owner decision points: one owner action is needed. Run `holden approve` once to sign the host's promises and cognitions. Only the owner can sign; this is a G-SITUATED requirement, not a preference.
