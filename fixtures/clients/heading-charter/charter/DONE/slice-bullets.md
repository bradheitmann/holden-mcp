# Done list — top-level proof bullets
## Proofs
1. P1 numbered bullet companion: numbered proof candidates retain their existing behavior.
- P2 dash bullet proof: a top-level dash item is an owner approval candidate with its exact text.
   - Reopen: this indented reopen sub-item must never become an independent owner candidate.
   - Fail-without mutant: this indented mutant sub-item must never become an owner candidate.
   - Test: this indented test sub-item must never become an independent owner candidate.
   4. D5 indented numbered sub-item: nesting prevents this line from becoming a candidate.
* P3 star bullet proof: a top-level star item is also an owner approval candidate.
   * Reopen: an indented star sub-item is supporting detail rather than a candidate.
## Gates
- D6 bulleted gate decoy: bullets outside proof blocks remain excluded.
