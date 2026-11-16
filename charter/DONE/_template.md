# Done list — <slice id> (written BEFORE dispatch; does not grow during it)
Base bytes: <commit/patch hash>
Proofs (each: name, what it guarantees, reopen step, fail-without mutant; a list item as below, or a `## Proof N — <name>` heading with the same lines):
1. P1 <name>: <what it guarantees, as one checkable sentence>
   - Reopen: <how the run is reopened and compared, e.g. a clean clone re-run>
   - Fail-without mutant: <the change that must turn the proof's test red>
   - Test: <the exact test name>
2. P2 <name>: <…>
   - Reopen: <…>
   - Fail-without mutant: <…>
Gates (exact commands):
-
Out of scope (recorded obligations, not fixed here):
-
Owner decision points:
-
