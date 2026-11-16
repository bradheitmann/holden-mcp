# Slice 3 QA attempt 2

**Verdict: accepted.** The frozen candidate remains `99a5bbfe14c935f470acec8bd6adac78f058bc20`. Attempt 2 corrected prerequisite verification and independently ran all seven required fail-without mutants. Each named test was red with its mutation applied and green after restoring the source.

## Checklist

- [x] Frozen bytes verified: manifest `842b97327d8a7222332f46ff7943c50efdbfe29af224e96f1af3343572a43601`, 176 entries; `cargo 1.95.0`.
- [x] Prerequisites verified from the main checkout with `charter receipt --verify-tree` against `/tmp` scratch checkouts: both slice-1 and slice-2 reported tree matches.
- [x] Host gate green, executed (attempt 1): 94 tests; fmt, clippy, forge export, pack loader and host guard passed.
- [x] Linux gate green, executed (attempt 1): 94 tests, same names/counts as host. VM guard tool availability is informational D3; host guard covers it and prior accepted slices record the same limitation.
- [x] M1–M7 each failed its named targeted test and passed after revert; exact commands and output summaries are in [evidence.md](evidence.md).
- [x] P1 checked all five doctrine roots against their cited canon sections.
- [x] P2 inspected exact byte slicing and quote verification; M2 also exercised the whitespace-acceptance mutant.
- [x] P3 inspected trusted-owner Ed25519 admission; M3 also exercised skipped verification.
- [x] P4 provenance chain mutant M4 failed as required.
- [x] P5 inspected NeedsOwnerWords boundary; M5 and the independent G-SITUATED mutant from attempt 1 failed as required.
- [x] P6 keyed lens draw mutant M6 failed as required.
- [x] P7 pack overlay reopen and M7 failure verified.
- [x] Own G-SITUATED mutant executed (attempt 1); archive checked.

D1 (required mutants missing) and D2 (wrong prerequisite verification method) are resolved in this attempt. No critical or high defects remain. The machine-readable result is [receipt.json](receipt.json).

Receipt validation: `~/open_protocols/charter/target/release/charter receipt qa/slice-3/receipt.json --verify-tree .` — tree matches the recorded freeze for slice-3, manifest `842b97327d8a7222332f46ff7943c50efdbfe29af224e96f1af3343572a43601`.
