# Synthdoc Schema v2.0

Companion to `LW-SYNTHESIS-PRIME.md`. A synthdoc is a typed **lens** over one original source
document; it is never a source of record. Downstream extraction cites the synthdoc's
`source_anchor` (which points at ORIGINAL source bytes), never the synthdoc prose.

## Top-level keys (complete allowed set)

| Key | Required | Meaning |
|---|---|---|
| `schema_version` | yes | Exact string `2.0`. |
| `source_document` | yes | `source_path`, `source_sha256`, `source_role: original_source`, `source_line_count`, `source_byte_length`. |
| `classification` | yes | v2 CCII primary category+item, secondary categories, confidence, rationale. |
| `facts` | yes | Material claims. Every item has a sibling `source_anchor`. Non-vacuity (I9) enforced. |
| `rules` | yes | Dependencies, gates, stop conditions, authority chains. Typed `edge_kind` + `from`/`to`. Anchored. |
| `nuances` | yes | Conditions, exceptions, intent, failure modes, contrastive notes. Anchored. |
| `nodes` | yes | Highest-value entities (typed `kind`), self-contained definition. Anchored. |
| `omissions` | yes | Category-expected-but-absent content, using the exact `not_found` sentinel. |
| `generation` | yes | Model, harness, instruction_set (`LW-SYNTHESIS-PRIME`), category_schema_version, seat. |

## `source_anchor` (sibling of every facts/rules/nuances/nodes item)

- `span_id`: exact grammar `span:N:start-end`.
- `start_line` / `end_line`: inclusive 1-based range matching `span_id`.
- `exact_quote`: **byte-exact substring** of the original source at that range.
- `quote_sha256`: SHA-256 of the `exact_quote` UTF-8 bytes (emitted as `"TO_COMPUTE"`; a deterministic
  verifier computes and confirms it — a fabricated hash is fail-closed).
- `origin`: exact string `original_source`. A synthdoc path, fabricated quote, or mismatched hash is fail-closed.

## Controlled vocabularies

- Node `kind`: `actor | artifact | code_candidate | concept | document | event | gate | governance_rule | metric | policy | process | system`.
- Rule `edge_kind`: `requires | blocks | verifies | produces | triggers | admits | supersedes | depends_on`.
- Nuance `kind`: `condition | contrastive_note | exception | failure_mode | intent`.

## Absence discipline

An omission uses `status: not_found` with an `item_id` and a statement explaining what the matched CCII
category expected but the source lacked. `not_found` is not an extracted fact and fabricates no anchor.
A skipped governance-bearing block is a critical recall-class omission.

An omission that LICENSES an empty claim section additionally carries the first-class typed field
`covers_empty_section: facts|rules|nuances|nodes` — one omission covers exactly one genuinely empty
section, the named section must actually be empty, and statement text alone licenses nothing (the
pre-W2-A leading `"<section>:"` prefix form is retired). Ordinary omissions omit the field.

**Empty-section licensing (enforced FORM, per-section I9).** A `facts`/`rules`/`nuances`/`nodes`
section may be empty ONLY when a governed omission TARGETS it. The deterministic FORM law is a
**leading typed prefix**: that omission's `statement` must begin with `"<section>:"` (e.g.
`"rules: the source states no dependency edges, so the rules section is empty."`). One statement can
carry only one leading prefix, so one omission targets exactly one section; an incidental
mid-statement mention licenses nothing. Rust checks only this FORM/coverage, never whether the
recorded absence is semantically justified (agents own meaning). NOTE: a fully typed
`covers_empty_section` omission field (a first-class schema target instead of an in-band prefix) is a
pending **W2-A** contract item; until it lands the leading-prefix FORM is the enforced surface.

## Verification

A deterministic post-step recomputes `source_sha256` and each `quote_sha256`, and confirms every
`exact_quote` is a byte-exact substring at its cited line range. Any anchor that fails is fail-closed
(not silently downgraded). This is what lets the fork ship synthdocs WITHOUT the originals: the
`exact_quote` is portable provenance.
