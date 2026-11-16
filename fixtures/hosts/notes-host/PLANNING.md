# Planning: Pocket Notes

Domain: Pocket Notes: a small command-line note keeper whose notes must survive interruptions and stay reachable by people and agents

Pocket Notes is a tiny, strongly typed Rust command-line tool that keeps
notes in a folder. People use it from a terminal; agents call the same
commands to file and read notes. It is the host project for holden-mcp's
end-to-end embedding test (slice-7): it is created from these committed
files at run time, so no private project is touched.

## Promises

The owner's words, one per line. Each line that starts with the promise
marker states exactly one promise.

- PROMISE: A saved note is never lost, even if the app closes halfway through saving.
- PROMISE: When an export fails, the person sees why it failed and how to retry.
- PROMISE: An agent can list every note through the public API without reading the screen.
- PROMISE: Deleting a note asks for confirmation before anything is removed.

## Not promises

Sync between machines, tags and search are ideas, not promises.
