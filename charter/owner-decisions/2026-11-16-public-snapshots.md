# Owner decision — 2026-11-16 public-snapshots

approved_by: owner

## Context

On 2026-11-16 the owner ordered: "prepare for open source, commit, push, make source open source" on his GitHub
account. A check before publishing found the tracked files clean (no owner personal data, no denylisted name, no
secret in any of 258 commits) but the commit history not: one commit added a home-directory path and a later commit
removed it, and seven older commits name a private project. The owner's standing rules forbid publishing either and
forbid rewriting history without his decision.

## Decision

Asked how to publish, the owner chose "Clean public copy": the development repository keeps its full history and
stays private, renamed `holden-mcp-dev`; a new public repository `holden-mcp` receives one snapshot commit of main's
tree each time a change is accepted. Nothing is rewritten and no history is published.

## Consequences

- Commit ids cited in `charter/` records refer to the development history and do not resolve in the public
  repository. CONTRIBUTING.md says so.
- A snapshot is published only when the guard and `charter check --strict` exit 0 on main, no tracked file matches an
  owner-PII pattern or holds a home-directory path, a secret scan of the tree is clean, and the snapshot message is
  clean. The publishing step refuses otherwise.
- Pull requests on the public repository are proposals; they are never merged on GitHub.
- The license is unchanged: PolyForm Noncommercial 1.0.0 (owner decision of 2026-11-04).
- First snapshot: public commit 44ebe648eb12, the tree of development commit 8bb500029913.
