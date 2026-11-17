# Owner decision — 2026-11-17 chrono-unmaintained

approved_by: owner

## Context

On 2026-11-17 RustSec published RUSTSEC-2026-0543: the `chrono` crate is unmaintained (all versions; no patched
version; the advisory is informational and reports no vulnerability). Its maintainer wound maintenance down and
recommends `jiff`; the advisory also names `time`.

holden-mcp's own code does not use `chrono`. It reaches the build only through `rmcp`, the official MCP SDK, and
through `schemars`'s `chrono04` feature, which `rmcp` turns on. Every `rmcp` release through 3.11.0 depends on
`chrono` unconditionally, so no upgrade removes it. Issue modelcontextprotocol/rust-sdk#1469 (opened 2026-11-17)
asks `rmcp` to move to `jiff` or `time`, or to make `chrono` optional.

`cargo xtask guard` runs `cargo deny check` and `cargo audit --deny warnings`, and until this decision both had no
advisory exceptions, so the guard refused every commit to main.

## Decision

Asked whether to take an exception or hold main until `rmcp` drops `chrono`, the owner chose the exception:
"I take the exception now so that we can unblock these and get these pushed."

- `deny.toml` `[advisories] ignore` and `.cargo/audit.toml` each name RUSTSEC-2026-0543, and nothing else.
- Every other advisory, including any future advisory against `chrono` itself, still refuses the guard.

## Removal

Remove both entries in the commit that moves `rmcp` to a release without `chrono`. If `rmcp` has not answered
#1469 by 2027-01-16, a planned slice evaluates `rust-mcp-sdk` (no direct `chrono`; uses `time`) as a replacement,
including a full check of its dependency tree.
