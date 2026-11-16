# holden-mcp

A master holdout-testing system. A holdout is a test the builder never sees. holden-mcp:

- defines holdout testing and its philosophical roots;
- derives situated cognitions (who meets the product, in what situation) from a project's planning evidence;
- generates sealed holdouts inside them;
- serves them through an MCP server that reveals each test's private random parameters only at test time, with a proof anyone can verify afterwards;
- measures its own catch rate on known controls;
- embeds holden agents into other projects for just-in-time holdout testing.

Rust only. macOS and Linux. No Docker, no Python.

## Owner-only key migration

If an installation predates OS-keystore custody, the **OWNER ONLY** runs this single command once, from the owner's terminal:

```sh
holden migrate-owner-key
```

This migrates the existing owner signing key from the default private-root owner store into macOS Keychain or the Linux passphrase-sealed keystore without changing `owner.pub`. Linux seals the key with Argon2id and XChaCha20-Poly1305; its passphrase is accepted only from an interactive TTY, never stdin, an environment variable or a file. Secret Service is not required. The command refuses headless execution and refuses to replace a different owner key. **Agents never run `holden migrate-owner-key` against the real owner store.** New disposable test installations can instead let `holden approve --create-owner-key` create the throwaway keystore item.

## Quickstart on macOS

Prerequisites: a Rust toolchain with Cargo, Git, and a supported harness (Claude, Codex, or Factory). The following demonstration uses a disposable private root and committed Pocket Notes fixture. Outside the demonstration, `holden init` without `--root` uses the real AGENT_PRIVATE_ROOT/XDG default; no path is hardcoded.

From a fresh clone, install the CLI, MCP server, confinement helper, and forge:

<!-- quickstart-test:setup -->
```sh
set -eu
export HOLDEN_QUICKSTART_ROOT="${HOLDEN_QUICKSTART_ROOT:-$(mktemp -d /tmp/holden-quickstart.XXXXXX)}"
printf 'quickstart root: %s\n' "$HOLDEN_QUICKSTART_ROOT"
export AGENT_PRIVATE_ROOT="$HOLDEN_QUICKSTART_ROOT/agent-private"
export CARGO_TARGET_DIR="$HOLDEN_QUICKSTART_ROOT/target"
export HOLDEN_HOST="$HOLDEN_QUICKSTART_ROOT/notes-host"
export HOLDEN_WORKSPACE="$PWD"

cargo install --locked --path crates/holden-cli --root "$HOLDEN_QUICKSTART_ROOT/install"
cargo install --locked --path crates/holden-mcp --root "$HOLDEN_QUICKSTART_ROOT/install"
cargo install --locked --path crates/holden-eval --bin holden-confine --root "$HOLDEN_QUICKSTART_ROOT/install"
cargo install --offline --locked --path modules/holden-forge --root "$HOLDEN_QUICKSTART_ROOT/install"
export PATH="$HOLDEN_QUICKSTART_ROOT/install/bin:$PATH"

holden init
printf '# synthetic quickstart denylist\nzz-quickstart-private-token\n' > "$AGENT_PRIVATE_ROOT/holden-mcp/denylist.txt"
holden demo-host --workspace "$HOLDEN_WORKSPACE" "$HOLDEN_HOST"
cat > "$HOLDEN_HOST/PLANNING.md" <<'EOF'
# Planning: Pocket Notes quickstart

Domain: Pocket Notes quickstart fixture

## Promises

- PROMISE: Deleting a note asks for confirmation before anything is removed.
EOF
holden embed "$HOLDEN_HOST" \
  --state "$HOLDEN_QUICKSTART_ROOT/agent-state" \
  --forge "$HOLDEN_QUICKSTART_ROOT/install/bin/forge" \
  --denylist "$AGENT_PRIVATE_ROOT/holden-mcp/denylist.txt" \
  --lens "$HOLDEN_WORKSPACE/crates/holden-cognition/fixtures/project-synthetic/lens"
```
<!-- quickstart-test:end -->

The fixture owner store is prepared separately in the automated test. Its real approval step is:

```sh
holden approve --host "$HOLDEN_HOST" --create-owner-key
```

Approval creates only a throwaway key in this demonstration's disposable Keychain and publishes `owner.pub`. Owner commands require the owner's terminal; a headless caller is refused.

Start the embedded agent in a second terminal with the same environment:

```sh
holden agent serve --host "$HOLDEN_HOST"
```

Wait for `holden agent: ready`. In the first terminal, make a real product commit. Its installed post-commit hook runs the just-in-time holdout once:

<!-- quickstart-test:commit -->
```sh
set -eu
printf '\nQuickstart check.\n' >> "$HOLDEN_HOST/README.md"
git -C "$HOLDEN_HOST" \
  -c user.name=holden-quickstart \
  -c user.email=quickstart@example.invalid \
  -c commit.gpgsign=false \
  commit --quiet --all -m "exercise the quickstart holdout"
```
<!-- quickstart-test:end -->

Finally, open `$HOLDEN_HOST` in the supported harness and run:

```text
/holden check
```

The slash command maps to `holden_check` on the host's implementer socket. A green result has `status: CHECK_COMPLETE` and one run whose `decision: pass`. If approval is pending, every workflow instead returns `OWNER_ACTION_REQUIRED` and prints the owner command without executing it. Use `/holden status` after approval to inspect measured status. An error, timeout, or pending owner action is never a pass.

Stop the foreground agent with Ctrl-C when finished. Remove only the temporary quickstart root printed by the setup.

## Embedding a host with declared crates

A host without a root `Cargo.toml` declares each real crate explicitly:

```sh
holden embed /path/to/host --manifest app/Cargo.toml
holden embed /path/to/host --manifest tools/a/Cargo.toml --manifest tools/b/Cargo.toml
```

Paths are host-relative `Cargo.toml` paths; absolute paths, `..`, symlink escapes, and Cargo packages outside the declared directory are refused. An undeclared non-root host is refused with `unsupported_stack: no root Cargo.toml; declare a crate with --manifest <relative-path> (repeatable)`. Each declared crate is built and tested independently and contributes its own binary entry points.

The default `--lens` corpus ships inside the installed `holden` executable, so the documented command works from any current directory. `--state` defaults to `<private root>/hosts/<name>`. The post-commit hook is installed in the repository's common git directory and is shared by every worktree. Planning is detected from `charter/` even when the host has no `PLANNING.md`. On a charter host, only numbered DONE proofs (longer than 40 characters) and `GUARANTEES.yaml` statements become candidates; underscore-prefixed DONE templates, headings and prose do not. The detection report prints the total, per-source counts and first 20 candidates with source lines, and writes the complete list to `candidate-promises.txt` in the agent state. Embed runs only `cargo metadata`; the post-commit JIT and W2 run `cargo build` and `cargo test` against an exported commit in a confined world, never in the host worktree.

## Build and test

Each slice's gate commands are listed in `charter/DONE/<slice>.md`. For the current convergence slice:

```sh
CARGO_TARGET_DIR=/tmp/holden-mcp-gate cargo xtask gate slice-12             # macOS or Linux host
CARGO_TARGET_DIR=/tmp/holden-mcp-gate cargo xtask gate slice-12 --linux-vm  # Linux proof in a Tart VM
HOLDEN_VM_ADDR=<vm address> cargo xtask gate slice-12 --linux-vm           # without calling tart
```

`HOLDEN_TARGET_BASE` (default `/tmp`) sets the base of every per-run build directory, on the host and in the VM. If `AGENT_PRIVATE_ROOT` is set, the VM driver forwards that absolute base and places the private denylist below it. Use `HOLDEN_TARGET_BASE` for the gate's own per-step directories; the `CARGO_TARGET_DIR` shown above is only the xtask launcher's directory.

## Start here

- Current state: `charter/STATE.md`
- Intent and guarantees: `charter/INTENT.md`, `charter/GUARANTEES.yaml`
- Architecture and slice plan: `docs/ARCHITECTURE.md`
- Agent rules: `AGENTS.md`

## Status

In development; no stable release yet. What works and what does not is recorded per slice in `charter/STATE.md`.
Known limits at this snapshot:

- The MCP server's socket paths can exceed the macOS limit of 104 bytes, so on macOS an agent may fail to connect to
  `holden mcp`. Linux is not affected. The fix is the next slice.
- Two repositories cloned under the same folder name share one agent state, and a post-commit holdout that does not
  run leaves no record. Both are fixed in the slice after that.

This public repository receives a snapshot at each accepted slice; see `CONTRIBUTING.md`. Security reports: `SECURITY.md`.

## License

PolyForm Noncommercial 1.0.0 (`PolyForm-Noncommercial-1.0.0`). See `LICENSE` and the Required Notice lines in `NOTICE`. Noncommercial use is permitted; commercial use requires a separate license from the licensor (contact through the repository's issue tracker). Owner decision: `charter/owner-decisions/2026-11-04-license-polyform-noncommercial.md`.
