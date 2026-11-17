//! slice-18h: short-named proofs, skipped proof lists and title boundaries.

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::path::PathBuf;

    use holden_cognition::adapters::done_source_accounting;
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

    fn texts(document: &DocumentPromises) -> Vec<&str> {
        document
            .promises
            .iter()
            .map(|promise| promise.text.as_str())
            .collect()
    }

    /// P1. Fail-without mutant: retain the 40-character drop for short
    /// items with sub-items; both exact source-backed candidates vanish.
    #[test]
    fn short_named_items_extend_through_their_claim_subitems() {
        let evidence = evidence("blindspot-charter");
        let document = document(&evidence, "charter/DONE/slice-short.md");
        let promises = &document.promises;
        assert_eq!(promises.len(), 3, "{:#?}", texts(document));
        let expected = [
            "D1:\n   - Reopen: the acceptance run remains available.\n   - Claim text: this first claim sub-item belongs to the short named proof.",
            "**D2:**\n   - Mutant: keep the short-name filter.",
        ];
        for (promise, expected) in promises.iter().zip(expected) {
            assert_eq!(promise.text, expected);
            let start = usize::try_from(promise.start).unwrap();
            let end = usize::try_from(promise.end).unwrap();
            assert_eq!(
                std::str::from_utf8(&document.document.bytes[start..end]).unwrap(),
                expected
            );
            promise.verify_against(&document.document).unwrap();
        }
        assert!(!promises.iter().any(|promise| {
            promise.text.contains("Test: short_named")
                || promise.text.contains("reopened: the record")
                || promise.text.contains("Fail-without: the candidate")
        }));
    }

    /// P2. Fail-without mutant: alter long-item extraction; the long item's
    /// text, covering span and content-derived id all change.
    #[test]
    fn long_items_are_unchanged_from_18g() {
        let current_evidence = evidence("blindspot-charter");
        let current_document = document(&current_evidence, "charter/DONE/slice-short.md");
        let expected = "P4 long item: its first line reaches the minimum and its complete continuation remains unchanged.\n   This continuation belongs to the long item exactly as it did before the short-name rule.";
        let start = current_document
            .document
            .bytes
            .windows(expected.len())
            .position(|window| window == expected.as_bytes())
            .unwrap();
        let old = Promise::slice(
            &current_document.document,
            start as u64,
            (start + expected.len()) as u64,
        )
        .unwrap();
        let actual = current_document
            .promises
            .iter()
            .find(|promise| promise.text.starts_with("P4 long item:"))
            .unwrap();
        assert_eq!(actual, &old);

        let old_evidence = evidence("wrapped-charter");
        let old_document = document(&old_evidence, "charter/DONE/slice-wrapped.md");
        assert_eq!(old_document.promises.len(), 4);
        assert_eq!(
            old_document.promises[0].text,
            "P1 wrapped numbered proof: the first line remains attached to every continuation line\n   so the owner reviews the complete promise rather than a truncated first line,\n   and the source span ends after this final continuation line."
        );
    }

    /// P3. Fail-without mutant: admit a short item without a sub-item; the
    /// exact candidate set gains `Tiny:`. The CLI P6 proof pins its reason.
    #[test]
    fn short_item_without_subitems_is_counted_too_short() {
        let evidence = evidence("blindspot-charter");
        let document = document(&evidence, "charter/DONE/slice-short.md");
        assert_eq!(document.promises.len(), 3, "{:#?}", texts(document));
        assert!(!texts(document).contains(&"Tiny:"));
        let accounting = done_source_accounting(&document.document).unwrap();
        assert_eq!(accounting.seen, 4);
        assert_eq!(accounting.taken, 3);
        assert_eq!(accounting.too_short, 1);
        assert_eq!(accounting.outside_proof_list, 0);
    }

    /// QA D1. Fail-without mutant: treat any indented line as a sub-item;
    /// the plain continuations wrongly rescue this short item from `too_short`.
    #[test]
    fn plain_continuations_are_not_bulleted_subitems() {
        let evidence = evidence("blindspot-charter");
        let document = document(&evidence, "charter/DONE/slice-plain-continuations.md");
        assert!(document.promises.is_empty(), "{:#?}", texts(document));
        let accounting = done_source_accounting(&document.document).unwrap();
        assert_eq!(accounting.seen, 1);
        assert_eq!(accounting.taken, 0);
        assert_eq!(accounting.too_short, 1);
        assert_eq!(accounting.outside_proof_list, 0);
    }

    /// P4. Fail-without mutant: keep the empty-labeled-block verdict; both
    /// title candidates remain hidden behind the later proof-evidence heading.
    #[test]
    fn labelless_blocks_fall_back_to_the_title_rule() {
        let evidence = evidence("blindspot-charter");
        let document = document(&evidence, "charter/DONE/slice-empty-label.md");
        assert_eq!(
            texts(document),
            [
                "P4 first title proof: the empty later proof heading does not hide this title item.",
                "P4 second title proof: the title fallback retains every consecutive numbered proof.",
            ]
        );
    }

    /// P5. Fail-without mutant: remove the reopened-sub-line fallback, or
    /// admit unmarked lists; the exact two-candidate set goes red.
    #[test]
    fn reopened_sublines_mark_proof_lists_anywhere() {
        let evidence = evidence("blindspot-charter");
        let document = document(&evidence, "charter/DONE/slice-anywhere.md");
        assert_eq!(
            texts(document),
            [
                "P5 first marked proof: a reopened sub-line identifies this list wherever it sits.\n   reopened: the acceptance record identifies the run and evidence digest.",
                "P5 second marked proof: consecutive items in the marked list remain independent candidates.",
            ]
        );
        for decoy in ["D1 steps", "D2 gates", "D3 schedule", "D4 landing"] {
            assert!(!texts(document).iter().any(|text| text.contains(decoy)));
        }
    }

    /// P6b. Fail-without mutant: let the level-one title's words open a
    /// proof block; four Design bullets join the five real candidates.
    #[test]
    fn title_words_never_open_a_proof_block() {
        let evidence = evidence("blindspot-charter");
        let document = document(&evidence, "charter/DONE/slice-title-proof.md");
        assert_eq!(document.promises.len(), 5, "{:#?}", texts(document));
        for (name, text) in ["first", "second", "third", "fourth", "fifth"]
            .iter()
            .zip(texts(document))
        {
            assert!(text.starts_with(&format!("P6b {name} proof:")));
        }
        assert!(
            !texts(document)
                .iter()
                .any(|text| text.contains("design bullet"))
        );
    }
}
