# Contributing to holden-mcp

Thank you for looking. This repository is developed under a written contract, the charter in `charter/`, and every
change goes through it. Read this page before opening an issue or a pull request.

## How the public repository relates to development

Development happens in a private repository. This public repository receives a snapshot commit each time a slice of
work is accepted, so its history is a series of snapshots. Commit ids cited in `charter/` records (accept commits,
dispatch bases, review receipts) refer to the development history and do not resolve here. The files, tests and
evidence in each snapshot are complete.

## Reporting a problem

Open an issue. Say what you ran, what you expected, what happened, and your OS and Rust version. For a holdout that
passed when it should have failed, or failed when it should have passed, include the output of `holden trace` if you
can share it. Do not post secrets, owner keys or the contents of a private root.

Security problems go through `SECURITY.md`, not the issue tracker.

## Proposing a change

1. Open an issue first and describe the behaviour you want. A change lands as a slice: a done list of numbered proofs
   written before the work starts (`charter/DONE/<slice>.md`), each with a step that reopens its result and a mutant
   that makes its test fail.
2. Pull requests are welcome as proposals. They are not merged on GitHub: an accepted change is re-applied through the
   development repository's gates and appears here in the next snapshot, with credit in the commit message.
3. Rust only. No Python, no JavaScript, no Docker. `unsafe` needs a stated reason. macOS and Linux both stay green.

## Building and testing

```sh
cargo build --workspace
cargo test --workspace
cargo xtask guard          # the repository's own safety checks
```

The toolchain is pinned in `rust-toolchain.toml`. Each slice's exact gate commands are in its done list. Some tests
install binaries into temporary directories and take several minutes.

## Rules that are not negotiable

- Tests never touch a real owner key, the OS keychain of the person running them, or a real private root.
- Nothing in the repository names a person's home directory, an email address or a private project.
- An agent never runs `holden approve` or `holden migrate-owner-key` against a real owner store. Those are the
  owner's commands.

## License of contributions

The project is under the PolyForm Noncommercial License 1.0.0 (`LICENSE`). By proposing a change you agree that it
may be distributed under that license and under any commercial license the licensor grants.
