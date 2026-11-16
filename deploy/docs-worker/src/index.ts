// Worker entry: bundles the committed documents and serves them read-only.
import readme from "../../../README.md";
import architecture from "../../../docs/ARCHITECTURE.md";
import { makeHandler } from "./app";

interface Env {
  readonly HOLDEN_COMMIT: string;
  readonly CANON_URL?: string;
}

export default {
  fetch(request: Request, env: Env): Response {
    return makeHandler({ readme, architecture }, { commit: env.HOLDEN_COMMIT, canonUrl: env.CANON_URL ?? "" })(request);
  },
} satisfies ExportedHandler<Env>;
