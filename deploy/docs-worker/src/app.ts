// Read-only documentation handler for holden-mcp (charter slice-16).
// Pure: the bundled entry (index.ts) and the tests both inject the documents.

export interface Docs {
  readonly readme: string;
  readonly architecture: string;
}

export interface Settings {
  /** The commit the documents were built from. */
  readonly commit: string;
  /** The published canon server, set at deploy time. Empty means not linked. */
  readonly canonUrl: string;
}

const READ_METHODS = new Set(["GET", "HEAD"]);

function baseHeaders(settings: Settings, contentType: string): Headers {
  return new Headers({
    "content-type": contentType,
    "x-holden-commit": settings.commit,
    "cache-control": "public, max-age=300",
    "x-content-type-options": "nosniff",
    "referrer-policy": "no-referrer",
  });
}

function respond(request: Request, body: string, headers: Headers, status = 200): Response {
  return new Response(request.method === "HEAD" ? null : body, { status, headers });
}

function indexPage(settings: Settings): string {
  const canon = settings.canonUrl
    ? `<li><a href="/canon">Holdout canon</a>: the read-only canon MCP server (schemas and doctrine)</li>`
    : "";
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>holden-mcp documentation</title>
<meta name="viewport" content="width=device-width, initial-scale=1"></head>
<body><main>
<h1>holden-mcp</h1>
<p>A holdout-testing system: tests the builder never sees, set inside situated cognitions, with a private random element revealed only through the MCP server at test time.</p>
<ul>
<li><a href="/readme">README and quickstart</a> (markdown)</li>
<li><a href="/architecture">Architecture</a> (markdown)</li>
${canon}
<li><a href="/llms.txt">llms.txt</a></li>
</ul>
<p>Read-only. Built from commit <code>${settings.commit}</code>. License: PolyForm Noncommercial 1.0.0.</p>
</main></body></html>
`;
}

function llmsTxt(settings: Settings): string {
  const lines = [
    "# holden-mcp",
    "",
    "> Holdout testing: sealed tests set inside situated cognitions, with moment-of-test parameters revealed only through the MCP server.",
    "",
    "- /readme: README with the quickstart (markdown)",
    "- /architecture: architecture and design decisions (markdown)",
  ];
  if (settings.canonUrl) {
    lines.push("- /canon: the read-only holdout canon MCP server");
  }
  lines.push("", `Built from commit ${settings.commit}.`, "");
  return lines.join("\n");
}

export function makeHandler(docs: Docs, settings: Settings): (request: Request) => Response {
  return (request: Request): Response => {
    if (!READ_METHODS.has(request.method)) {
      const headers = baseHeaders(settings, "text/plain; charset=utf-8");
      headers.set("allow", "GET, HEAD");
      return respond(request, "method not allowed: this endpoint is read-only\n", headers, 405);
    }
    const path = new URL(request.url).pathname;
    switch (path) {
      case "/":
      case "/index.html": {
        const headers = baseHeaders(settings, "text/html; charset=utf-8");
        headers.set("content-security-policy", "default-src 'none'; base-uri 'none'; form-action 'none'");
        return respond(request, indexPage(settings), headers);
      }
      case "/readme":
        return respond(request, docs.readme, baseHeaders(settings, "text/markdown; charset=utf-8"));
      case "/architecture":
        return respond(request, docs.architecture, baseHeaders(settings, "text/markdown; charset=utf-8"));
      case "/llms.txt":
        return respond(request, llmsTxt(settings), baseHeaders(settings, "text/plain; charset=utf-8"));
      case "/canon":
        if (settings.canonUrl) {
          const headers = baseHeaders(settings, "text/plain; charset=utf-8");
          headers.set("location", settings.canonUrl);
          return respond(request, `see ${settings.canonUrl}\n`, headers, 302);
        }
        break;
      default:
        break;
    }
    return respond(request, "not found\n", baseHeaders(settings, "text/plain; charset=utf-8"), 404);
  };
}
