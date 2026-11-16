# Security

holden-mcp seals tests so that the builder of a product cannot read them, and signs approvals with an owner key. A
flaw in either is a security problem.

## What to report privately

- A way for an implementer (an agent or a person without the owner key) to read a sealed holdout, its private random
  parameters before test time, or the private root.
- A way to forge, replay or bypass an owner approval, or to make `holden ledger verify` accept an altered ledger.
- A way out of the confinement that runs a host's build and tests, or a path that lets a host write outside its
  allowed directories.
- A leak of key material, in output, logs, state files or errors.

## How to report

Use GitHub's private vulnerability reporting on this repository (Security tab, "Report a vulnerability"). Do not open a
public issue for the items above. Include the commit you tested, your OS, and the smallest set of steps that shows the
problem. You will get an answer in the report thread.

## What is in scope

The crates in this repository at the latest snapshot on `main`, on macOS and Linux. The custody model and its stated
limits are in `charter/INTENT.md` and `docs/ARCHITECTURE.md`. The main one: in personal mode the implementer and the
holden agent share one OS user, so the separation of roles comes from which socket a caller reaches and from the file
system, not from a second user id. An attack that needs only that shared user is a documented limit, not a new
vulnerability. A report showing the limit is wider than documented is welcome.

## Supported versions

Only the latest snapshot on `main` is supported. There are no backports.
