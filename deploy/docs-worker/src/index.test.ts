// Proof tests for charter slice-16 (charter/DONE/slice-16.md).
import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, readdirSync, existsSync } from "node:fs";
import { tmpdir, homedir } from "node:os";
import { join } from "node:path";
import { makeHandler } from "./app";

const repo = join(import.meta.dir, "..", "..", "..");
const readme = readFileSync(join(repo, "README.md"), "utf8");
const architecture = readFileSync(join(repo, "docs", "ARCHITECTURE.md"), "utf8");
const settings = { commit: "test-commit", canonUrl: "https://canon.example.invalid/" };
const handle = makeHandler({ readme, architecture }, settings);
const url = (path: string): string => `https://docs.example.invalid${path}`;

describe("slice-16", () => {
  test("rejects non-read methods", () => {
    for (const method of ["POST", "PUT", "PATCH", "DELETE", "OPTIONS"]) {
      const response = handle(new Request(url("/readme"), { method, body: method === "OPTIONS" ? null : "x" }));
      expect(response.status).toBe(405);
      expect(response.headers.get("allow")).toBe("GET, HEAD");
    }
    expect(handle(new Request(url("/nope"))).status).toBe(404);
    const head = handle(new Request(url("/readme"), { method: "HEAD" }));
    expect(head.status).toBe(200);
    expect(head.body).toBeNull();
  });

  test("serves committed docs byte for byte", async () => {
    const r = handle(new Request(url("/readme")));
    expect(r.headers.get("content-type")).toBe("text/markdown; charset=utf-8");
    expect(r.headers.get("x-holden-commit")).toBe("test-commit");
    expect(await r.text()).toBe(readme);
    expect(await handle(new Request(url("/architecture"))).text()).toBe(architecture);
    const canon = handle(new Request(url("/canon")));
    expect(canon.status).toBe(302);
    expect(canon.headers.get("location")).toBe(settings.canonUrl);
  });

  test("bundle carries no private identifiers", () => {
    const out = mkdtempSync(join(tmpdir(), "holden-docs-bundle-"));
    const build = Bun.spawnSync(["bunx", "wrangler", "deploy", "--dry-run", "--outdir", out], {
      cwd: join(import.meta.dir, ".."),
      stdout: "pipe",
      stderr: "pipe",
    });
    expect(build.exitCode).toBe(0);
    const bundle = readdirSync(out)
      // Every uploaded module (index.js and the bundled markdown); the source map is not uploaded.
      .filter((name) => !name.endsWith(".map"))
      .map((name) => readFileSync(join(out, name), "utf8"))
      .join("\n");
    expect(bundle).toContain(readme.slice(0, 200));
    expect(/\/Users\/[A-Za-z]|\/home\/[a-z][a-z0-9_-]*\//.test(bundle)).toBe(false);
    expect(bundle.includes(homedir())).toBe(false);
    expect(/-----BEGIN [A-Z ]*PRIVATE KEY-----/.test(bundle)).toBe(false);
    const denylist = process.env.HOLDEN_DENYLIST
      ?? join(process.env.AGENT_PRIVATE_ROOT ?? join(homedir(), ".local", "share", "agent-private"), "holden-mcp", "denylist.txt");
    expect(existsSync(denylist)).toBe(true);
    const terms = readFileSync(denylist, "utf8")
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && !line.startsWith("#"));
    expect(terms.length).toBeGreaterThan(0);
    const lowered = bundle.toLowerCase();
    // Count only; never print a matched term.
    const hits = terms.filter((term) => lowered.includes(term.toLowerCase())).length;
    expect(hits).toBe(0);
  }, 120_000);
});
