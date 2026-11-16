# canon/

- `holden/`: the S-DOCTRINE canon files, byte-identical to the pin. `holden canon verify` (run from the repository root) checks them by default; `--root <dir>` verifies another copy, such as one in the private root.
- `CANON-PIN.txt`: the S-DOCTRINE canon pin, matching S-CANON-MCP `check_canon_lock` (`docs/evidence/canon-mcp-pin-2026-11-04.txt`). The four canon files are committed in `holden/` (owner decision D-08, 2026-11-04, `charter/owner-decisions/2026-11-04-canon-committed.md`).
- `lens-manifest.sha256`: the manifest of the private S-LENS corpus (hashes only). Deltas: `LENS-DELTAS.md`.

Both files use the pin format `holden.pin/1`, parsed by `holden-canon` (`crates/holden-canon/src/pin.rs`):

```text
# comment lines start with '#'; blank lines are ignored
format: holden.pin/1                 (required, first non-comment line)
canon_version: <text>                (required, once)
validator_hash: <64 lowercase hex>   (optional, once)
<64 lowercase hex>  <relative/path>  (one per file, sha256sum layout)
```

Paths are relative, `/`-separated, with no `.`, `..` or empty components. `holden canon verify [--root <dir>] [--pin <file>]` recomputes every file's SHA-256 and refuses any differing, missing or extra file, naming it (exit 1). An absent root is the typed limitation `canon_absent` (exit 3), never a pass.
