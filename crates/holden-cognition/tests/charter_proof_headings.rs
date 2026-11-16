//! slice-18f: charter proof lists under Markdown headings, including
//! top-level bullets, without admitting implementation or indented detail.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;

    use holden_cognition::{ClientPromises, read_promises};

    fn evidence(fixture: &str) -> ClientPromises {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/clients")
            .join(fixture);
        read_promises(&root).unwrap()
    }

    fn candidates(evidence: &ClientPromises) -> Vec<(String, String)> {
        evidence
            .spans()
            .into_iter()
            .map(|(document, promise)| {
                promise.verify_against(document).unwrap();
                (document.name.clone(), promise.text.clone())
            })
            .collect()
    }

    /// P1. Fail-without mutant: recognize only a bare `Proofs` line; all
    /// four Markdown heading forms lose their numbered proof candidates.
    #[test]
    fn proof_blocks_start_at_proofs_lines_and_markdown_headings() {
        let actual = candidates(&evidence("heading-charter"));
        let expected = [
            (
                "slice-proofs.md",
                "P1 plain heading proof: the source-backed candidate survives a standard Markdown proof heading.",
            ),
            (
                "slice-proofs.md",
                "P2 plain heading trace: a second numbered candidate remains attached to its exact source bytes.",
            ),
            (
                "slice-revision.md",
                "P1 revised heading proof: a parenthesized revision label does not hide the numbered proof.",
            ),
            (
                "slice-revision.md",
                "P2 revised heading trace: revision metadata stays outside the candidate source span.",
            ),
            (
                "slice-fixed.md",
                "P1 fixed group proof: a heading with proof inside its label starts the candidate block.",
            ),
            (
                "slice-fixed.md",
                "P2 fixed group trace: every numbered item in the group retains its original wording.",
            ),
            (
                "slice-singular.md",
                "P1 singular heading proof: the singular word Proof starts a Markdown proof block.",
            ),
            (
                "slice-singular.md",
                "P2 singular heading trace: an em dash and title do not become part of the candidate.",
            ),
            (
                "slice-bullets.md",
                "P1 numbered bullet companion: numbered proof candidates retain their existing behavior.",
            ),
            (
                "slice-bullets.md",
                "P2 dash bullet proof: a top-level dash item is an owner approval candidate with its exact text.",
            ),
            (
                "slice-bullets.md",
                "P3 star bullet proof: a top-level star item is also an owner approval candidate.",
            ),
            (
                "slice-bold.md",
                "P1 ordinary companion proof: the ordinary numbered item remains beside the bold-named form.",
            ),
            (
                "slice-bold.md",
                "**bold-name.** The bold-named proof retains its markers and exact source-backed candidate text.",
            ),
        ];
        assert_eq!(
            actual.len(),
            expected.len(),
            "unexpected candidates: {actual:#?}"
        );
        for (file, text) in expected {
            let name = format!("charter/DONE/{file}");
            assert!(
                actual.contains(&(name, text.to_owned())),
                "missing {file}: {text}"
            );
        }
        for decoy in ["D1 top-level", "D2 non-proof", "D3 indented", "D4 steps"] {
            assert!(
                !actual.iter().any(|(_, text)| text.contains(decoy)),
                "admitted {decoy}"
            );
        }
    }

    /// P1 bold-named form. The same bare-`Proofs`-only mutant removes this
    /// candidate; its text and one-based source line pin the complete span.
    #[test]
    fn bold_named_proof_item_under_a_heading_is_a_candidate() {
        let evidence = evidence("heading-charter");
        let document = evidence
            .documents
            .iter()
            .find(|document| document.document.name == "charter/DONE/slice-bold.md")
            .unwrap();
        let bold = document
            .promises
            .iter()
            .find(|promise| promise.text.starts_with("**bold-name.**"))
            .unwrap();
        assert_eq!(
            bold.text,
            "**bold-name.** The bold-named proof retains its markers and exact source-backed candidate text."
        );
        bold.verify_against(&document.document).unwrap();
        let start = usize::try_from(bold.start).unwrap();
        let line = document.document.bytes[..start]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
            + 1;
        assert_eq!(line, 4);
    }

    /// P2. Fail-without mutant: drop top-level `-`/`*` candidacy or admit
    /// indented sub-items; the exact three-item candidate set goes red.
    #[test]
    fn top_level_bullets_are_candidates_indented_subitems_are_not() {
        let evidence = evidence("heading-charter");
        let actual = candidates(&evidence);
        let document = evidence
            .documents
            .iter()
            .find(|document| document.document.name == "charter/DONE/slice-bullets.md")
            .unwrap();
        let texts = document
            .promises
            .iter()
            .map(|promise| promise.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "P1 numbered bullet companion: numbered proof candidates retain their existing behavior.",
                "P2 dash bullet proof: a top-level dash item is an owner approval candidate with its exact text.",
                "P3 star bullet proof: a top-level star item is also an owner approval candidate.",
            ]
        );
        for excluded in [
            "Reopen:",
            "Fail-without mutant:",
            "Test:",
            "D5 indented",
            "D6 bulleted",
        ] {
            assert!(
                !actual.iter().any(|(_, text)| text.contains(excluded)),
                "admitted {excluded}"
            );
        }
    }

    /// P3. Fail-without mutant: accept numbered items outside proof blocks;
    /// Steps/Gates/Out-of-scope decoys join the old byte-exact candidate set.
    #[test]
    fn decoy_sections_stay_excluded_and_older_fixtures_hold() {
        let numbered = candidates(&evidence("numbered-charter"));
        let expected = [
            (
                "charter/GUARANTEES.yaml",
                "A reviewer can reopen every accepted proof from its recorded source bytes.",
            ),
            (
                "charter/GUARANTEES.yaml",
                "Every owner approval traces to the exact guarantee statement that was reviewed.",
            ),
            (
                "charter/DONE/slice-a.md",
                "P1 reopen proof: a reviewer reopens the recorded source and compares its bytes.\n   This continuation is supporting prose rather than another approval candidate.",
            ),
            (
                "charter/DONE/slice-a.md",
                "P2 trace proof: every accepted result identifies the planning line it came from.",
            ),
            (
                "charter/DONE/slice-b.md",
                "P1 refusal proof: an unapproved candidate cannot host a holdout check.",
            ),
            (
                "charter/DONE/slice-b.md",
                "P2 repeat proof: a second extraction returns the identical ordered candidates.",
            ),
            (
                "charter/DONE/slice-c.md",
                "P1 boundary proof: source paths and line numbers survive the complete report path.",
            ),
            (
                "charter/DONE/slice-c.md",
                "P2 exclusion proof: headings, prose and short lines never become owner candidates.",
            ),
        ];
        assert_eq!(
            numbered.len(),
            expected.len(),
            "old candidate set changed: {numbered:#?}"
        );
        for (name, text) in expected {
            assert!(
                numbered.contains(&(name.to_owned(), text.to_owned())),
                "missing old candidate {name}: {text}"
            );
        }
        for decoy in ["P9 gate step", "P10 gate exclusion", "P11 scope exclusion"] {
            assert!(
                !numbered.iter().any(|(_, text)| text.contains(decoy)),
                "admitted {decoy}"
            );
        }

        let prose = candidates(&evidence("prose-charter"));
        assert_eq!(
            prose,
            [(
                "charter/DONE/slice-1.md".to_owned(),
                "P1 source proof: the tool keeps  every owner-approved planning quote available to a successor.".to_owned(),
            )]
        );
    }
}
