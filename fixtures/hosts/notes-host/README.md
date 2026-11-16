# Pocket Notes (host fixture for holden embed)

A small command-line note keeper, used as the host project of holden-mcp's
end-to-end embedding test (slice-7).

The repository is created at run time (`holden_embed::fixture::create_host`,
or `holden demo-host <dir>`):

1. the product's source is the committed notes fixture product
   (`fixtures/products/notes`, without its `variants/`): `Cargo.toml`,
   `Cargo.lock`, `notes.toml` and `src/`;
2. this directory adds the host's own planning document (`PLANNING.md`,
   which states the owner's promises) and this README;
3. a `.gitignore` ignores `target/`;
4. everything is committed once, with a fixed identity and date.

```text
notes --store <dir> import          # records "name<TAB>body<LF>" on stdin
notes --store <dir> list            # one note name per line, sorted
notes --store <dir> get <name>      # the note's body, byte for byte
notes --store <dir> delete <name> [--yes]
notes --store <dir> export <path>
```

Then `holden embed <repo>` installs the holden agent, the owner runs
`holden approve` once, `holden agent serve` runs the agent, and every
`git commit` runs a just-in-time sealed holdout through the post-commit hook.
