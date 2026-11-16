{
  "schema_version": "charter.receipt/1",
  "slice": "slice-18b",
  "verifies": "slice-18b attempt-2 independent adversarial QA: frozen identity and bounded scope, fresh P1-P5 proof execution and fail-without controls, repaired numbered-short-line and successful-report bounds, template/refusal/path/per-source pins, independent Linux VM gate and parity, real-repository regression, canonical safety and strict checks",
  "candidate_commit": "7c179fd2a5777cad3f134b09b930c7aca8abc512",
  "frozen": {
    "manifest_sha256": "fd6785fe31ac3b8057d7518280b384a4627c13190972e06230dc722ac5326d16",
    "head": "7c179fd2a5777cad3f134b09b930c7aca8abc512"
  },
  "dispatch": {
    "base_commit": "7c179fd2a5777cad3f134b09b930c7aca8abc512",
    "brief_sha256": null
  },
  "author_model": "deepseek-v4.1-flash (declared_by: droid-mission harness)",
  "verifier_model": "minimax-m3.1-flash (declared_by: droid-mission harness)",
  "verdict": "accepted",
  "checks": [
    {
      "id": "C01",
      "description": "FROZEN IDENTITY PASS. A fresh clone was empty under git status --porcelain --untracked-files=all at 7c179fd2a5777cad3f134b09b930c7aca8abc512 with tree 87ea297699b78572d5fafd94806123cce2417b28. DONE SHA-256 and dispatch done_sha256 both equal 7163a9c4b9a8dac346a308dcb69be84a0e2d0d0b8491584b823ae48ed6c5469c; dispatch base_commit is the same tip. The charter.freeze/2 record names that commit/tree, manifest fd6785fe31ac3b8057d7518280b384a4627c13190972e06230dc722ac5326d16 and 432 entries. Independent normalized comparison excluded precisely the 46 tracked prior charter/.freeze records, then found exactly 432 Git entries and zero path/mode/object/type differences; all 432 blob SHA-256 values matched.",
      "classification": "executed"
    },
    {
      "id": "C02",
      "description": "BOUNDED SCOPE PASS. git diff --check 8316d5b..7c179fd exited 0. The diff has 22 granted paths plus inherited plans/2026-11-06-integration/dispatch-graph.json, which is byte-identical to origin/main; no outside path exists. AMENDMENT 1's lib.rs hunks are all inside inline mod tests. xtask/src/gate.rs changes only SLICES from length 22 to 23 and adds slice-18b. The repair ec6884a..7c179fd changes exactly the three granted files crates/holden-cli/tests/charter_promises.rs, crates/holden-cli/tests/embed_report.rs and fixtures/clients/numbered-charter/charter/DONE/slice-b.md; its diff check also exited 0.",
      "classification": "executed"
    },
    {
      "id": "C03",
      "description": "P1 + D-QA-18B-1 REPAIR PASS. Restoring paragraph extraction made charter_yields_numbered_proofs_and_guarantees RED at the exact candidate-count assertion (exit 101, 19s); exact revert was GREEN 1/1 (exit 0, 18s). Replacing only the production chars().count() > 40 guard with true made the same P1 test RED at candidate promises 8 (exit 101, 17s); exact revert was GREEN 1/1 (exit 0, 16s). The fixture now has sequential numbered lines 1, 2 and 3; line 8 is `3. P3 short: do not approve this.` (33 full-line characters, 30 promise-body characters), and GREEN P1 explicitly excludes the exact source path/line candidate. Template inclusion was separately RED (exit 101, 18s) and its exact revert GREEN 1/1 (exit 0, 20s).",
      "classification": "executed"
    },
    {
      "id": "C04",
      "description": "P2 + D-QA-18B-2 REPAIR PASS. Removing .take(20) only from print_detection_report made the embed_report suite RED: successful_charter_report_stays_bounded observed all 28 candidates instead of min(28,20)=20, while the other four tests passed (exit 101, 20s). Exact revert was GREEN 5/5 (exit 0, 23s). The success proof runs a real successful 28-candidate charter embed and pins total and per-source counts, the first-20 occurrence bound, stdout below 64 KB, the relative full-list path, absence of P22 from output and all 28 lines including P22 in the state file. The shape comparison now independently counts candidate occurrences on success and refusal rather than relying on collapsed repeated fields.",
      "classification": "executed"
    },
    {
      "id": "C05",
      "description": "REPORT PIN COVERAGE PASS. Removing the refusal printer's cap made report_prints_first_20_and_writes_the_full_list RED with 1,319,378-byte stderr (exit 101, 20s); exact revert was GREEN 1/1 (exit 0, 20s). Making the full-list path absolute made that test RED at the relative-path assertion (exit 101, 19s); exact revert was GREEN 1/1 (exit 0, 18s). Zeroing success-path per-source counts made successful_charter_report_stays_bounded RED at the slice-c count assertion (exit 101, 24s); exact revert was GREEN 1/1 (exit 0, 21s). Together with C03-C04, template exclusion, numbered <=40-character exclusion, bounded success and refusal output, total/per-source counts, first-20 output and relative full-list persistence all have executed teeth.",
      "classification": "executed"
    },
    {
      "id": "C06",
      "description": "P3 PLANNING COMPATIBILITY PASS. Replacing explicit Markdown extraction with prose extraction made planning_hosts_yield_the_same_candidates RED at the PLANNING.md candidate-count assertion (exit 101, 17s); exact revert was GREEN 1/1 (exit 0, 19s). The proof covers existing prose-markdown expectations and a charter-shaped host whose PLANNING.md retains Markdown precedence and verbatim marked candidates. The independent VM record also carries this proof [ok].",
      "classification": "executed"
    },
    {
      "id": "C07",
      "description": "P4 REFUSAL REPORT PASS. Dropping the report from the refusal branch made refused_embed_prints_the_shortened_report RED at the missing planning-process assertion (exit 101, 19s); exact revert was GREEN 1/1 (exit 0, 21s). The proof pins report-before-refusal ordering, total and per-source counts, all eight candidates for the small fixture, the relative full-list path and absence of the fixture machine root. The independent VM record also carries this proof [ok].",
      "classification": "executed"
    },
    {
      "id": "C08",
      "description": "P5 HOST/LINUX PARITY + REAL-REPOSITORY PASS. Reopened orchestrator records agent-state/runs/20261115T211627Z-macos.json (green, 206 names, SHA-256 e750a59790a26958e688dcce158b20df82618923fc19a3808595ffbb20230e1d) and agent-state/runs/20261115T211620Z-linux-vm.json (green_with_declared_limitations, 205 names, SHA-256 283be26cb72e3b15bdaacb05d5f382713875f246943907833bf01bb0550df6a5; VM original 20261115T211134Z-linux.json). Independent QA Boat gate exited 0 in 349s and produced agent-state/runs/20261115T213812Z-linux-vm.json (VM original 20261115T213311Z-linux.json), same commit, tree_clean true, x86_64, green_with_declared_limitations, 205/0/0, SHA-256 f984db51c08f899be783e5773b71e475cd342ba89725e0026c980d56a49ef548. QA and orchestrator Linux name sets are identical. Independent comm derives 202 shared names plus exactly four standing macOS-only names (migrate_removes_legacy_key_after_verify, readme_install_covers_holden_mcp, readme_quickstart_leads_to_green_check, written_mcp_json_drives_w1_through_w4) and three standing Linux-only names (non_tty_caller_cannot_approve, sealed_roundtrip_uses_argon2id_and_xchacha20poly1305, unseal_requires_interactive_tty). P1-P4 and successful_charter_report_stays_bounded are [ok] in all three records; every recorded name is [ok]. The four real-repository names w1_real_promises_verbatim, two_commits_different_params_audit_verified, planted_regression_fails_w2_w3_names_consequence_only and telemetry_reports_catch_rates_with_samples are [ok] in the QA record. Only cargo_deny_absent, cargo_audit_absent and charter_absent are declared on Linux, covered by the host checks.",
      "classification": "executed"
    },
    {
      "id": "C09",
      "description": "SAFETY + STRICT + CLEANLINESS PASS. Canonical charter build 2775000 safety exited 0 over 478 tracked files with zero refused findings; its five standing allow-entry warnings are non-refusing. Final safety after all exact reverts also exited 0. Canonical charter check --strict exited 0 in 1s with only standing accepted-slice cost-row warnings. Every valid mutant exited 101, every exact revert passed git diff --exit-code and its test, and no mutant was committed. Before writing these QA governance records the clone had zero tracked or untracked status entries. Mutant scratch was absent after every literal-path deletion. Own Boat scratch was removed by literal paths; protected holden-1000 socket directories and older slice litter were not touched.",
      "classification": "executed"
    },
    {
      "id": "C10",
      "description": "QA HARNESS FAIL-STATE PRESERVED. The first length-mutant invocation used an unqualified test name together with --exact, ran zero tests and exited 0 in 11s. It is invalid evidence and is excluded from every proof result. Re-running the same live mutant with the exact fully qualified tests:: name executed one test and produced C03's required exit 101. No product gate overlapped this setup mistake. D-QA-PROC-2 records the invalid invocation without laundering a zero-test run as a pass.",
      "classification": "executed"
    },
    {
      "id": "C11",
      "description": "VERDICT ACCEPTED. Both attempt-1 refusal defects are closed by demonstrated red/green controls, all DONE P1-P5 proofs were executed and reopened, the additional behavior-clause mutants in C05 have teeth, independent Linux and orchestrator records agree, and real-repository regression is 4/4. No unresolved slice candidate defect was found in the assigned verification surface; this conclusion rests on C01-C10 rather than an unsupported claim of no findings. The only open item is the pre-existing canonical-tool model-family vocabulary issue D-QA-TOOL-2, not a slice behavior defect.",
      "classification": "executed"
    },
    {
      "id": "C12",
      "description": "CANONICAL REOPEN + RECEIPT VALIDATION PASS. Initial accepted-receipt validation identified the canonical structured-new_defects and per-proof reopen-attestation requirements; both were supplied without changing DONE or weakening a check. charter reopen slice-18b --proof all --run-id 20261115T213812Z-linux-vm --evidence <independent-VM-record> exited 0 and wrote charter/.freeze/reopen/slice-18b.json with five entries, the pinned DONE/manifest hashes and evidence SHA-256 f984db51c08f899be783e5773b71e475cd342ba89725e0026c980d56a49ef548. The final exact sanctioned command charter receipt qa/slice-18b/receipt-7c179fd.md --allow-same-family qa/slice-18b/mission-qa-seat-applicability.md exited 0 from the QA clone root. The applicability note quotes the owner decision's MiniMax validator seat verbatim from slice-7 QA on, names slice-18b, grants nothing and carries no approved_by field.",
      "classification": "executed"
    }
  ],
  "defects": [
    {
      "id": "D-QA-18B-1",
      "severity": "medium",
      "status": "resolved",
      "description": "Attempt 1 found that the numbered <=40-character exclusion had no teeth. The repair adds sequential fixture item `3. P3 short: do not approve this.` and an exact exclusion assertion. Removing only the production length predicate now makes P1 RED (exit 101, 17s), and exact revert is GREEN 1/1 (exit 0, 16s)."
    },
    {
      "id": "D-QA-18B-2",
      "severity": "medium",
      "status": "resolved",
      "description": "Attempt 1 found that the success-path first-20 bound had no teeth. The repair adds a real successful 28-candidate embed proof with occurrence, output-size, source-count and full-list assertions, and makes shape comparison count occurrences. Removing only the success printer's .take(20) now makes the new test RED (exit 101, 20s), and exact revert is GREEN 5/5 (exit 0, 23s)."
    },
    {
      "id": "D-QA-PROC-2",
      "severity": "info",
      "status": "resolved",
      "description": "One initial QA mutant command combined an unqualified filter with --exact and ran zero tests (exit 0, 11s). It was rejected as evidence, the live mutant was rerun with the fully qualified test name, and the valid run went RED as required. No product result was inferred from the zero-test invocation."
    },
    {
      "id": "D-QA-TOOL-2",
      "severity": "info",
      "status": "open",
      "description": "Canonical charter build 2775000 omits MiniMax from its built-in model-family table. The owner explicitly assigned MiniMax and DeepSeek as different families from slice-7 QA on. The slice-specific applicability note records that existing ruling without granting a new bar; receipt validation uses the sanctioned --allow-same-family lift."
    }
  ],
  "new_defects": [
    {
      "id": "D-QA-PROC-2",
      "severity": "info",
      "status": "resolved",
      "description": "One initial QA mutant command combined an unqualified filter with --exact and ran zero tests (exit 0, 11s). It was rejected as evidence, rerun with the fully qualified test name, and the valid live-mutant run went RED as required."
    }
  ],
  "proofs_reopened": [
    "P1 paragraph mutant RED exit 101 in 19s, exact revert GREEN 1/1 exit 0 in 18s; length-guard mutant RED exit 101 in 17s, exact revert GREEN 1/1 exit 0 in 16s; template mutant RED exit 101 in 18s, exact revert GREEN 1/1 exit 0 in 20s; independent Linux [ok]",
    "P2 refusal print-all mutant RED exit 101 in 20s at 1,319,378-byte stderr, exact revert GREEN 1/1 exit 0 in 20s; success-cap mutant RED exit 101 in 20s with 28 rather than 20 occurrences, exact revert GREEN 5/5 exit 0 in 23s; absolute-path mutant RED exit 101 in 19s, revert GREEN in 18s; zeroed per-source counts RED exit 101 in 24s, revert GREEN in 21s; independent Linux [ok]",
    "P3 planning-extraction mutant RED exit 101 in 17s; exact revert GREEN 1/1 exit 0 in 19s; independent Linux [ok]",
    "P4 dropped-refusal-report mutant RED exit 101 in 19s; exact revert GREEN 1/1 exit 0 in 21s; independent Linux [ok]",
    "P5 independent comm compared 206 macOS and 205 Linux unique all-ok names in orchestrator and QA records: 202 shared plus exactly the standing four host-only and three Linux-only names; P1-P4 and the repaired success-bound test green on every platform",
    "Independent Boat gate exit 0 in 349s, 205/0/0 green_with_declared_limitations; real-repository regression 4/4 [ok]; canonical safety exit 0 and strict check exit 0; verdict ACCEPTED with D-QA-18B-1 and D-QA-18B-2 resolved"
  ]
}
