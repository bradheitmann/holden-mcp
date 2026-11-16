# S-LENS import deltas (holden.lens/1)

The S-LENS corpus is derived from published design books, so it is never committed. `holden canon import-lens --from <S-LENS registry dir> --to <private root>/corpora/lens` reads only the five JSON registries, converts them to the typed model in `holden-canon` (`crates/holden-canon/src/lens.rs`), applies the deltas below, and writes the corpus plus a `holden.pin/1` manifest into the private root. The committed `canon/lens-manifest.sha256` is that manifest, byte for byte: hashes only.

Import run of 2026-11-04: 280 concepts (278 original, 2 meta), 24 situated cognitions, 7 retired concept ids. Manifest listing digest `3613ee8a88cb6131aaf094806e48e1a7ad2b5801c7c20f39808386e34a0514af`.

Verify on a machine that holds the private corpus (otherwise the command reports the typed limitation `canon_absent`, exit 3, never a pass):

```sh
holden canon verify --root "$AGENT_PRIVATE_ROOT/holden-mcp/corpora/lens/corpus" --pin canon/lens-manifest.sha256
```

| Id | Delta | Applied on 2026-11-04 | Evidence (hashes and counts only) |
|---|---|---|---|
| D-LENS-1 | Timestamp-free, position-free ids. Source sequence numbers, source ids and positional cognition ids are dropped. Every id is the SHA-256 of the record's canonical bytes (`holden.lens/1`). | yes | 280 concept and 24 cognition source ids dropped |
| D-LENS-2 | Timestamped provenance dropped: creation times, model, harness, repository and commit fields. | yes | 282 records (280 concepts plus 2 meta records) |
| D-LENS-3 | The one stereotyped situated cognition is rewritten into a stereotype-free situation that keeps its cognitive load (two concurrent time-critical demands, fragmented attention). The match is by hash of the old text; the old text is not kept here. | yes | old text sha256 `0c1c9e705edd6cc4d16017e00d823cde5d83ee1ec9ab4d9deb5630c3ecfb6063`; new text sha256 `5c7d2ac302fc3da10a4e43a2273a80173383fce4f4b820a120a05002b8312929`; new cognition id `2b70588eb35afd22af50cd6deb984ceec0d357a5983585e1cfa9897bb6ef5fce` |
| D-LENS-4 | No executable script. The source's draw script and any other non-registry entry are never read, copied or run; the corpus holds data only. | yes | 2 source-directory entries not read |
| D-LENS-5 | Unseeded draw state dropped. Draws come from the VRF in `holden-seal`, not from the source's unseeded randomizer. | yes | 0 draw-state entries in the source |

The same deltas are recorded per import in the private corpus as `deltas.json`, which the manifest pins.
