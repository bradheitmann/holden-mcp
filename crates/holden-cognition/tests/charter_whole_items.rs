//! slice-18g: a charter proof candidate is its whole numbered or bulleted
//! item, including indented continuations and optional lower-case numeric
//! suffixes, while existing single-line candidates retain their exact ids.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;

    use holden_cognition::{ClientPromises, DocumentPromises, Promise, read_promises};

    fn evidence(fixture: &str) -> ClientPromises {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/clients")
            .join(fixture);
        read_promises(&root).unwrap()
    }

    fn document<'a>(evidence: &'a ClientPromises, name: &str) -> &'a DocumentPromises {
        evidence
            .documents
            .iter()
            .find(|document| document.document.name == name)
            .unwrap()
    }

    /// P1. Fail-without mutant: emit only the item's first line; the exact
    /// verbatim multiline text and covering source span both go red.
    #[test]
    fn wrapped_items_are_one_candidate_with_a_covering_span() {
        let evidence = evidence("wrapped-charter");
        let document = document(&evidence, "charter/DONE/slice-wrapped.md");
        assert_eq!(document.promises.len(), 4, "{:#?}", document.promises);
        let wrapped = &document.promises[0];
        assert_eq!(
            wrapped.text,
            "P1 wrapped numbered proof: the first line remains attached to every continuation line\n   so the owner reviews the complete promise rather than a truncated first line,\n   and the source span ends after this final continuation line."
        );
        let start = usize::try_from(wrapped.start).unwrap();
        let end = usize::try_from(wrapped.end).unwrap();
        let raw = std::str::from_utf8(&document.document.bytes[start..end]).unwrap();
        assert_eq!(
            raw,
            "P1 wrapped numbered proof: the first line remains attached to every continuation line\n   so the owner reviews the complete promise rather than a truncated first line,\n   and the source span ends after this final continuation line."
        );
        assert_eq!(raw, wrapped.text);
        wrapped.verify_against(&document.document).unwrap();

        let texts = document
            .promises
            .iter()
            .map(|promise| promise.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "P1 wrapped numbered proof: the first line remains attached to every continuation line\n   so the owner reviews the complete promise rather than a truncated first line,\n   and the source span ends after this final continuation line.",
                "P2 next numbered proof: the following top-level item remains an independent candidate.",
                "P3 wrapped bullet proof: a bulleted item also owns its indented continuation\n  through this final line, with exactly one space joining the two source lines.",
                "P4 star bullet after blank: a later top-level item is still a separate candidate.",
            ]
        );
        for decoy in [
            "Reopen:",
            "Fail-without mutant:",
            "Indented prose after a blank",
            "D1 gate decoy",
        ] {
            assert!(
                !texts.iter().any(|text| text.contains(decoy)),
                "admitted {decoy}"
            );
        }
    }

    /// P2. Fail-without mutant: strip `**` from a bold item name; the exact
    /// source-backed candidate text goes red.
    #[test]
    fn bold_names_keep_their_markers() {
        let evidence = evidence("heading-charter");
        let document = document(&evidence, "charter/DONE/slice-bold.md");
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
    }

    /// P3. Fail-without mutant: alter single-line extraction; an older text,
    /// span, or content-derived id no longer matches these byte-exact pins.
    #[test]
    fn single_line_candidates_are_unchanged_across_fixtures() {
        let expected = [
            (
                "numbered-charter",
                "charter/GUARANTEES.yaml",
                "A reviewer can reopen every accepted proof from its recorded source bytes.",
            ),
            (
                "numbered-charter",
                "charter/GUARANTEES.yaml",
                "Every owner approval traces to the exact guarantee statement that was reviewed.",
            ),
            (
                "numbered-charter",
                "charter/DONE/slice-a.md",
                "P1 reopen proof: a reviewer reopens the recorded source and compares its bytes.\n   This continuation is supporting prose rather than another approval candidate.",
            ),
            (
                "numbered-charter",
                "charter/DONE/slice-a.md",
                "P2 trace proof: every accepted result identifies the planning line it came from.",
            ),
            (
                "numbered-charter",
                "charter/DONE/slice-b.md",
                "P1 refusal proof: an unapproved candidate cannot host a holdout check.",
            ),
            (
                "numbered-charter",
                "charter/DONE/slice-b.md",
                "P2 repeat proof: a second extraction returns the identical ordered candidates.",
            ),
            (
                "numbered-charter",
                "charter/DONE/slice-c.md",
                "P1 boundary proof: source paths and line numbers survive the complete report path.",
            ),
            (
                "numbered-charter",
                "charter/DONE/slice-c.md",
                "P2 exclusion proof: headings, prose and short lines never become owner candidates.",
            ),
            (
                "prose-charter",
                "charter/DONE/slice-1.md",
                "P1 source proof: the tool keeps  every owner-approved planning quote available to a successor.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-proofs.md",
                "P1 plain heading proof: the source-backed candidate survives a standard Markdown proof heading.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-proofs.md",
                "P2 plain heading trace: a second numbered candidate remains attached to its exact source bytes.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-revision.md",
                "P1 revised heading proof: a parenthesized revision label does not hide the numbered proof.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-revision.md",
                "P2 revised heading trace: revision metadata stays outside the candidate source span.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-fixed.md",
                "P1 fixed group proof: a heading with proof inside its label starts the candidate block.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-fixed.md",
                "P2 fixed group trace: every numbered item in the group retains its original wording.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-singular.md",
                "P1 singular heading proof: the singular word Proof starts a Markdown proof block.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-singular.md",
                "P2 singular heading trace: an em dash and title do not become part of the candidate.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-bullets.md",
                "P1 numbered bullet companion: numbered proof candidates retain their existing behavior.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-bullets.md",
                "P2 dash bullet proof: a top-level dash item is an owner approval candidate with its exact text.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-bullets.md",
                "P3 star bullet proof: a top-level star item is also an owner approval candidate.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-bold.md",
                "P1 ordinary companion proof: the ordinary numbered item remains beside the bold-named form.",
            ),
            (
                "heading-charter",
                "charter/DONE/slice-bold.md",
                "**bold-name.** The bold-named proof retains its markers and exact source-backed candidate text.",
            ),
        ];
        let mut actual_count = 0usize;
        for (fixture, name, text) in expected {
            let evidence = evidence(fixture);
            let document = document(&evidence, name);
            let start = document
                .document
                .bytes
                .windows(text.len())
                .position(|window| window == text.as_bytes())
                .unwrap();
            let end = start + text.len();
            let id = Promise::slice(&document.document, start as u64, end as u64)
                .unwrap()
                .id;
            let promise = document
                .promises
                .iter()
                .find(|promise| promise.text == text)
                .unwrap_or_else(|| panic!("missing old candidate {name}: {text}"));
            assert_eq!(
                (promise.start, promise.end, promise.id),
                (start as u64, end as u64, id)
            );
            promise.verify_against(&document.document).unwrap();
        }
        for fixture in ["numbered-charter", "prose-charter", "heading-charter"] {
            actual_count += evidence(fixture).spans().len();
        }
        assert_eq!(
            actual_count, 22,
            "an older fixture gained or lost candidates"
        );
    }

    /// P5. Fail-without mutant: close a heading-opened block at prose or a
    /// lower-level subheading; the numbered items after them disappear.
    #[test]
    fn heading_blocks_survive_prose_subheadings_and_fences() {
        let evidence = evidence("wrapped-charter");
        let document = document(&evidence, "charter/DONE/slice-heading-blocks.md");
        let texts = document
            .promises
            .iter()
            .map(|promise| promise.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "P5 before subheading: the first numbered item survives leading prose in a heading-opened block.",
                "P5 after fence: the next numbered item survives the subheading, code fence, and table.",
                "P5 final item: another lower-level subheading still does not terminate the proof block.",
            ]
        );
        for decoy in ["D5 fenced decoy", "D6 table decoy", "D7 gate decoy"] {
            assert!(
                !texts.iter().any(|text| text.contains(decoy)),
                "admitted {decoy}"
            );
        }
        for promise in &document.promises {
            promise.verify_against(&document.document).unwrap();
        }
    }

    /// P6. Fail-without mutant: close a label block at its leading prose, or
    /// keep it open after top-level narrative following its first item.
    #[test]
    fn label_blocks_tolerate_leading_prose_and_still_exclude_decoys() {
        let evidence = evidence("wrapped-charter");
        let document = document(&evidence, "charter/DONE/slice-label-blocks.md");
        let texts = document
            .promises
            .iter()
            .map(|promise| promise.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "P6 wrapped label item: numbered proof text after the wrapped label and leading prose is admitted.",
                "P6 plain label first item: a plain Proofs colon label tolerates prose before its first item.",
                "P6 plain label second item: consecutive numbered items remain independent candidates.",
            ]
        );
        for decoy in ["D8 gate decoy", "D9 trailing decoy"] {
            assert!(
                !texts.iter().any(|text| text.contains(decoy)),
                "admitted {decoy}"
            );
        }
    }

    /// P7. Fail-without mutant: ignore an unlabeled title list, or continue
    /// taking numbered items after the first second-level heading.
    #[test]
    fn unlabeled_title_lists_are_proofs_other_headings_are_not() {
        let evidence = evidence("wrapped-charter");
        let document = document(&evidence, "charter/DONE/slice-unlabeled.md");
        let texts = document
            .promises
            .iter()
            .map(|promise| promise.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "P7 first title proof: an unlabeled DONE file admits the top-level numbered item under its title.",
                "P7 second title proof: consecutive title-list items remain independent source-backed candidates.",
            ]
        );
        for decoy in ["D10 gate decoy", "D11 schedule decoy", "D12 landing decoy"] {
            assert!(
                !texts.iter().any(|text| text.contains(decoy)),
                "admitted {decoy}"
            );
        }
    }

    /// P4. Fail-without mutant: require an all-digit marker; 4b, 4c and the
    /// following proof 5 disappear from the exact four-candidate pin.
    #[test]
    fn lettered_numbers_are_candidates_and_do_not_end_the_block() {
        let evidence = evidence("wrapped-charter");
        let document = document(&evidence, "charter/DONE/slice-lettered.md");
        let texts = document
            .promises
            .iter()
            .map(|promise| promise.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            texts,
            [
                "P4 base numbered proof: the ordinary numeric marker remains a candidate.",
                "P4 first lettered proof: a lower-case suffix does not terminate the proof block.",
                "P4 second lettered proof: consecutive lettered numeric markers remain candidates.",
                "P5 following numbered proof: extraction continues after the lettered items.",
            ]
        );
        for promise in &document.promises {
            promise.verify_against(&document.document).unwrap();
        }
    }
}
