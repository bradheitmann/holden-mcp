# Done list — plain continuations are not sub-items
## Proofs
1. Served bundles carry the rule.
   `cargo test --lib served_bundles_carry_the_rule` → `1 passed`.
   The test resolves the served phase text and asserts the non-prescription sentence.
   Reopened: the result line records one passing test.
   Fail-without: restore the original product text and rerun the same command.
