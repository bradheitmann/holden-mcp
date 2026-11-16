# Done list — slice-16 (written BEFORE dispatch; does not grow during it)
Base bytes: main at 402da03d0e75e5229b39059cea4b2a125248d579
Scope: a public, read-only Cloudflare Worker that serves holden-mcp's own documentation (README with the quickstart, docs/ARCHITECTURE.md) and links to the already-published canon server. The owner chose "Publish docs" on 2026-11-11. No code, keys, sealed holdouts, plans or private paths are served. Source: `deploy/docs-worker/` (strict TypeScript, Bun for tests, Wrangler for deploy).
Proofs:
1. P1 read-only: every method other than GET and HEAD gets 405 with an `Allow: GET, HEAD` header. Unknown paths get 404. No route accepts input that changes state, and the Worker has no bindings: no KV, D1, R2, Durable Objects, secrets or vars apart from the build commit.
   - Reopen: `bun test` in deploy/docs-worker from a fresh clone. After deploy, `curl -X POST` against the live URL returns 405.
   - Fail-without mutant: make POST fall through to the GET handler. The test `rejects non-read methods` must fail.
   - Test: `deploy/docs-worker/src/index.test.ts` :: `rejects non-read methods`
2. P2 byte-exact docs: the /readme and /architecture pages serve the committed README.md and docs/ARCHITECTURE.md byte for byte as text/markdown. Every response carries an `x-holden-commit` header naming the commit the bytes were built from.
   - Reopen: `bun test` compares the served bodies with the files read from disk. After deploy, `curl <url>/readme | shasum -a 256` equals `shasum -a 256 README.md` at the commit in the header.
   - Fail-without mutant: trim or alter the served README. The test `serves committed docs byte for byte` must fail.
   - Test: `deploy/docs-worker/src/index.test.ts` :: `serves committed docs byte for byte`
3. P3 nothing private ships: the deployed bundle contains no private identifier, absolute home path, key material or sealed artefact. `cargo xtask guard` passes on the candidate, and a denylist scan of the built bundle finds nothing.
   - Reopen: `npx wrangler deploy --dry-run --outdir <tmp>`, then scan the bundle with the private denylist and the home-path pattern. Run `cargo xtask guard`.
   - Fail-without mutant: add a file containing a denylisted term to the served set. The test `bundle carries no private identifiers` must fail.
   - Test: `deploy/docs-worker/src/index.test.ts` :: `bundle carries no private identifiers`
4. P4 strict and live: `tsc --noEmit` passes with `strict: true`. The Worker is deployed to the owner's account. The live /, /readme, /architecture, /llms.txt and /canon routes respond as in P1 and P2, and /canon points at the canon server.
   - Reopen: `bunx tsc --noEmit`; curl each live route and check its status and headers.
   - Fail-without mutant: set `strict` to false and add an implicit any. With strict restored, `tsc --noEmit` must refuse it.
   - Test: `bunx tsc --noEmit` and the live checks above.
Gates (exact commands):
- `cd deploy/docs-worker && bun install --frozen-lockfile && bunx tsc --noEmit && bun test`
- `cargo xtask guard` and `charter check --strict` at the repository root.
Out of scope (recorded obligations, not fixed here):
- A custom domain. The workers.dev subdomain is used.
- Publishing any canon change. The canon server is unchanged.
Owner decision points:
- None. The owner chose "Publish docs" on 2026-11-11.
