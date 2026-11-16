# Fixture product: `notes` (SYNTHETIC)

A small, dependency-free Rust CLI that keeps notes. It is the real product
`holden-eval` builds and observes at its boundary (slice-4). It is its own
Cargo workspace and is excluded from the holden-mcp workspace.

The tests never build this directory in place. They create a fixture git
repository at run time (`holden_eval::fixture::FixtureRepo`):

1. commit `keeps`: every file here except `variants/`;
2. one commit per directory under `variants/`, each made on top of `keeps`
   by copying that overlay's files over it.

| Commit | Behaviour |
|---|---|
| `keeps` | correct: every complete record is saved; a record cut off by an interruption is ignored |
| `breaks` | silently drops data: a note whose body is longer than 64 bytes is skipped, and `import` still reports success |
| `refuses` | errors: an interrupted import marks the store damaged and every later read is refused |
| `escapes` | hostile: tries to write outside its world, open a network socket, flood its output, hang and grow a file without end; its build script records its environment |
| `reconfig` | changes the public config `notes.toml` only |
| `keeps-alt` | correct, through a different implementation (synced saves in a `notes/` folder); a known-good control (slice-5) |
| `torn-write` | broken where the probe cannot see: a note is rewritten in place, so a crash mid-write loses it; a real miss (slice-5) |
| `import-noop` | does nothing for the saving promise: `import` reports success and saves nothing (slice-5 do-nothing control) |
| `list-noop` | does nothing for the listing promise: `list` succeeds and prints nothing (slice-5 do-nothing control) |
| `export-noop` | does nothing for the export promise: `export` writes nothing and reports success (slice-5 do-nothing control) |
| `delete-noop` | does nothing for the delete promise: `delete` asks nothing, removes nothing and reports success (slice-5 do-nothing control) |
| `hangs` | `import` saves, then never exits; only an enforced run-time limit ends it (slice-5 never-again control) |
| `sneaks` | keeps every promise at its boundary, but after a delete refusal or a failed export it tries to write outside its world (slice-5 P6(a)) |
| `peeks` | keeps every promise at its boundary, but on every import it tries to read a sealed file planted next to its world and reports what it read (slice-7 P5) |

Each commit is built from source in a scratch worktree at that commit and
run under the confinement backend, with a scrubbed environment.
