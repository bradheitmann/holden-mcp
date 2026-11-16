# Done list — whole-item wrapping
## Proofs
1. P1 wrapped numbered proof: the first line remains attached to every continuation line
   so the owner reviews the complete promise rather than a truncated first line,
   and the source span ends after this final continuation line.
   - Reopen: this supporting sub-item is outside the candidate.
   - Fail-without mutant: this supporting sub-item is outside the candidate too.
2. P2 next numbered proof: the following top-level item remains an independent candidate.
- P3 wrapped bullet proof: a bulleted item also owns its indented continuation
  through this final line, with exactly one space joining the two source lines.

   Indented prose after a blank line is not part of either preceding candidate.
* P4 star bullet after blank: a later top-level item is still a separate candidate.
   * Reopen: an indented star sub-item remains supporting detail.
## Gates
5. D1 gate decoy: numbered items outside the proof block stay excluded.
