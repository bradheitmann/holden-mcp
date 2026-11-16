//! holden-gen: approved promise × approved cognition × keyed lens draw →
//! sealed holdout plan (docs/ARCHITECTURE.md §3.3).
//!
//! - [`draw`]: the keyed lens draw, HMAC-SHA-256 under a sealed draw key,
//!   weighted by corpus weights and replayable for audit.
//! - [`plan`]: the typed plan, its provenance chain, "needs owner words"
//!   items, and sealing through `holden-seal`.

pub mod draw;
pub mod plan;

pub use draw::{DrawCorpus, DrawInputs, LensDraw, draw as draw_lens, replay};
pub use plan::{GenPlan, Generated, NeedsOwnerWords, PublicCard, generate, seal_plan, unseal_plan};

/// The committed synthetic fixture project's lens corpus, relative to this
/// crate's manifest directory.
pub const FIXTURE_LENS: &str = "../holden-cognition/fixtures/project-synthetic/lens";

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    use holden_canon::Doctrine;
    use holden_cognition::{
        Approval, HostableCognition, PlanningDocument, Promise, candidates, extract_promises,
    };
    use holden_seal::{
        DrawKey, FileKeyStore, KeyMaterial, KeyName, KeyStore, OwnerKey, SealKey, UnsealError,
        plan_commit,
    };
    use sha2::{Digest, Sha256};

    use super::*;

    fn crate_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("holden-gen-test-{name}-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    struct Fixture {
        doctrine: Doctrine,
        doc: PlanningDocument,
        promises: Vec<Promise>,
        hosted: Vec<HostableCognition>,
        corpus: DrawCorpus,
        draw_key: DrawKey,
    }

    /// The synthetic project: every promise extracted, the first two
    /// candidates of each approved by an owner key from a file key store,
    /// and a draw key from the same store.
    fn fixture(name: &str) -> Fixture {
        let doctrine = Doctrine::holden();
        let doc = PlanningDocument::read(
            &crate_dir().join("../holden-cognition/fixtures/project-synthetic/PLANNING.md"),
            "PLANNING.md",
        )
        .unwrap();
        let promises = extract_promises(&doc).unwrap();
        let store = FileKeyStore::open(scratch(name).join("keys")).unwrap();
        let (owner_name, draw_name) = (
            KeyName::new("owner").unwrap(),
            KeyName::new("draw").unwrap(),
        );
        store
            .put(&owner_name, &KeyMaterial::from_bytes([0x51; 32]))
            .unwrap();
        store
            .put(&draw_name, &KeyMaterial::from_bytes([0x52; 32]))
            .unwrap();
        let owner = OwnerKey::load(&store, &owner_name).unwrap();
        let draw_key = DrawKey::load(&store, &draw_name).unwrap();
        let mut hosted = Vec::new();
        for p in &promises {
            for c in candidates(&doctrine, p, None).into_iter().take(2) {
                let approval = Approval::sign(&owner, &c);
                hosted.push(
                    HostableCognition::admit(&doc, p, &c, Some(&approval), &owner.public())
                        .unwrap(),
                );
            }
        }
        let corpus = DrawCorpus::load(&crate_dir().join(FIXTURE_LENS)).unwrap();
        Fixture {
            doctrine,
            doc,
            promises,
            hosted,
            corpus,
            draw_key,
        }
    }

    fn inputs(h: &HostableCognition, epoch: u64) -> DrawInputs {
        DrawInputs {
            promise: h.promise().id,
            cognition: h.cognition().id,
            epoch,
        }
    }

    fn gen_for(f: &Fixture, h: &HostableCognition, epoch: u64) -> Generated {
        let d = draw_lens(&f.draw_key, &f.corpus, &inputs(h, epoch)).unwrap();
        let approved: Vec<&Promise> = f.promises.iter().collect();
        generate(&f.doctrine, h, &approved, &f.corpus, &d).unwrap()
    }

    /// slice-3 P4: every plan generated for the fixture carries a chain
    /// promise → cognition → roots → lens concepts, and every link resolves:
    /// the promise is verbatim in the planning document, the cognition is
    /// the approved one for that promise, each root is a doctrine root whose
    /// citations resolve in the committed canon, and each lens concept is in
    /// the corpus and was drawn by the recorded, replayable draw.
    #[test]
    fn every_plan_traces_promise_cognition_root() {
        let f = fixture("trace");
        f.doctrine
            .verify(&crate_dir().join("../../canon/holden"))
            .unwrap();
        assert!(!f.hosted.is_empty());
        for h in &f.hosted {
            let g = gen_for(&f, h, 0);
            let p = &g.plan.provenance;
            assert_eq!(g.plan.id, g.plan.computed_id());
            let promise = f.promises.iter().find(|x| x.id == p.promise).unwrap();
            promise.verify_against(&f.doc).unwrap();
            assert_eq!(p.source, f.doc.sha256());
            assert_eq!(p.cognition, h.cognition().id);
            assert_eq!(h.cognition().promise, p.promise);
            assert!(!p.roots.is_empty(), "the plan names no root");
            assert_eq!(p.roots, h.cognition().roots);
            for r in &p.roots {
                let root = f.doctrine.root(*r).unwrap();
                assert!(!root.citations.is_empty());
            }
            assert!(!p.lens_concepts.is_empty());
            assert_eq!(p.lens_concepts, p.draw.picks);
            for c in &p.lens_concepts {
                assert!(f.corpus.concept(c).is_some());
            }
            replay(&f.draw_key, &f.corpus, &p.draw).unwrap();
            assert_eq!(p.draw.inputs, inputs(h, 0));
            let approved: BTreeSet<_> = f.promises.iter().map(|x| x.id).collect();
            for check in g.plan.checks() {
                assert!(approved.contains(&check.promise));
            }
            let check_ids: Vec<&str> = g.plan.checks().iter().map(|c| c.id.as_str()).collect();
            assert!(check_ids.contains(&g.plan.broken_variant.must_fail.as_str()));
        }
    }

    /// slice-3 P4: generation is deterministic for the same draw, and
    /// different draws (epochs or keys) give different plans.
    #[test]
    fn different_draws_different_plans() {
        let f = fixture("draws");
        let h = &f.hosted[0];
        assert_eq!(gen_for(&f, h, 3), gen_for(&f, h, 3));
        let plans: Vec<GenPlan> = (0..8).map(|e| gen_for(&f, h, e).plan).collect();
        let ids: BTreeSet<_> = plans.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), plans.len(), "every draw gives a distinct plan");
        let lens_sets: BTreeSet<Vec<_>> = plans
            .iter()
            .map(|p| p.provenance.lens_concepts.clone())
            .collect();
        assert!(lens_sets.len() > 1, "draws choose different lenses");
        let steps: BTreeSet<Vec<String>> = plans
            .iter()
            .map(|p| p.steps.iter().map(|s| s.action.clone()).collect())
            .collect();
        assert!(steps.len() > 1, "different lenses give different steps");
        let other_key = DrawKey::new(KeyMaterial::from_bytes([0x99; 32]));
        let d = draw_lens(&other_key, &f.corpus, &inputs(h, 0)).unwrap();
        let approved: Vec<&Promise> = f.promises.iter().collect();
        let g = generate(&f.doctrine, h, &approved, &f.corpus, &d).unwrap();
        assert_ne!(g.plan.id, plans[0].id);
    }

    /// slice-3 P4: the plan is sealed with holden-seal; the public card names
    /// the why (promise, cognition, roots) and the commit, never the lens; the
    /// right key unseals the identical plan and anything else is refused.
    #[test]
    fn plan_is_sealed() {
        let f = fixture("sealed");
        let g = gen_for(&f, &f.hosted[0], 1);
        let key = SealKey::new(KeyMaterial::from_bytes([0x61; 32]));
        let (sealed, card) = seal_plan(&key, &g.plan).unwrap();
        let commit = plan_commit(&sealed);
        assert_eq!(card.plan_commit, hex::encode(commit.0));
        assert_eq!(card.plan, g.plan.id);
        assert_eq!(card.roots, g.plan.provenance.roots);
        let card_json = serde_json::to_string(&card).unwrap();
        for c in &g.plan.provenance.lens_concepts {
            assert!(!card_json.contains(&c.to_hex()), "the card names a lens");
            let name = &f.corpus.concept(c).unwrap().name;
            assert!(!card_json.contains(name.as_str()));
        }
        let text = g.plan.provenance.lens_concepts[0].to_hex();
        assert!(
            !sealed
                .ciphertext
                .windows(text.len())
                .any(|w| w == text.as_bytes()),
            "the sealed bytes carry plaintext"
        );
        let back = unseal_plan(&key, &sealed, &commit).unwrap();
        assert_eq!(back, g.plan);
        assert_eq!(back.id, back.computed_id());
        let wrong = SealKey::new(KeyMaterial::from_bytes([0x62; 32]));
        assert_eq!(
            unseal_plan(&wrong, &sealed, &commit),
            Err(UnsealError::TagRefused)
        );
        let mut tampered = sealed.clone();
        tampered.ciphertext[0] ^= 0x01;
        assert_eq!(
            unseal_plan(&key, &tampered, &commit),
            Err(UnsealError::CommitMismatch)
        );
    }

    /// slice-3 P5: a drawn lens whose implied property no approved promise
    /// covers yields a NeedsOwnerWords item and no check for that property;
    /// every check expects an approved promise quoted verbatim.
    #[test]
    fn lens_implied_promise_goes_to_needs_owner_words() {
        let f = fixture("owner-words");
        let uncovered: Vec<_> = f
            .corpus
            .entries
            .iter()
            .filter(|e| {
                !f.promises.iter().any(|p| {
                    let t = p.text.to_ascii_lowercase();
                    e.implies
                        .terms
                        .iter()
                        .all(|x| t.contains(&x.to_ascii_lowercase()))
                })
            })
            .map(|e| e.concept)
            .collect();
        assert!(
            !uncovered.is_empty(),
            "the fixture lens implies an uncovered property"
        );
        let mut seen_uncovered = 0;
        let mut seen_covered = 0;
        for h in &f.hosted {
            for epoch in 0..8 {
                let g = gen_for(&f, h, epoch);
                for pick in &g.plan.provenance.lens_concepts {
                    let entry = f.corpus.entry(pick).unwrap();
                    let item = g.needs_owner_words.iter().find(|n| &n.concept == pick);
                    if uncovered.contains(pick) {
                        seen_uncovered += 1;
                        let item = item.expect("an uncovered property needs owner words");
                        assert_eq!(item.property, entry.implies.property);
                        assert_eq!(item.promise, h.promise().id);
                        assert_eq!(item.cognition, h.cognition().id);
                    } else {
                        seen_covered += 1;
                        assert!(item.is_none(), "a covered property needs no owner words");
                    }
                }
                for check in g.plan.checks() {
                    for e in &f.corpus.entries {
                        assert!(
                            !check.expect.contains(&e.implies.property)
                                && !check.observe.contains(&e.implies.property),
                            "a check was emitted for a lens-implied property"
                        );
                    }
                    let quoted = f
                        .promises
                        .iter()
                        .find(|p| p.id == check.promise)
                        .expect("every check enforces an approved promise");
                    assert!(check.expect.contains(&format!("\"{}\"", quoted.text)));
                }
                assert!(!g.plan.broken_variant.mutation.contains("LENS PROPERTY"));
            }
        }
        assert!(seen_uncovered > 0, "no draw picked an uncovered lens");
        assert!(seen_covered > 0, "no draw picked a covered lens");
    }

    /// slice-3 P6: the draw is keyed. Under two keys the same inputs give
    /// different MACs (and, across epochs, different picks), and no MAC is the
    /// unkeyed SHA-256 of its message.
    #[test]
    fn lens_draw_requires_key() {
        let f = fixture("keyed");
        let h = &f.hosted[0];
        let other = DrawKey::new(KeyMaterial::from_bytes([0x53; 32]));
        let mut picks_differ = false;
        for epoch in 0..16 {
            let i = inputs(h, epoch);
            let a = draw_lens(&f.draw_key, &f.corpus, &i).unwrap();
            let b = draw_lens(&other, &f.corpus, &i).unwrap();
            for (n, (ma, mb)) in a.macs.iter().zip(&b.macs).enumerate() {
                assert_ne!(ma, mb, "two keys gave the same MAC");
                let plain = hex::encode(Sha256::digest(draw::draw_message(&i, n as u64)));
                assert_ne!(ma, &plain, "the draw is the unkeyed SHA-256");
                let mac = f.draw_key.mac(&draw::draw_message(&i, n as u64));
                assert_eq!(ma, &hex::encode(mac));
            }
            picks_differ |= a.picks != b.picks;
        }
        assert!(picks_differ, "the key never changed a pick");
    }

    /// slice-3 P6: a recorded draw replays from its inputs under the draw
    /// key; a changed pick, epoch or key does not.
    #[test]
    fn lens_draw_replays() {
        let f = fixture("replay");
        let h = &f.hosted[1];
        let d = draw_lens(&f.draw_key, &f.corpus, &inputs(h, 5)).unwrap();
        let wire = serde_json::to_string(&d).unwrap();
        let back: LensDraw = serde_json::from_str(&wire).unwrap();
        replay(&f.draw_key, &f.corpus, &back).unwrap();
        assert_eq!(back.picks.len(), draw::DRAW_PICKS);
        let unique: BTreeSet<_> = back.picks.iter().collect();
        assert_eq!(
            unique.len(),
            back.picks.len(),
            "a draw never repeats a concept"
        );
        let mut swapped = d.clone();
        swapped.picks.reverse();
        assert!(replay(&f.draw_key, &f.corpus, &swapped).is_err());
        let mut later = d.clone();
        later.inputs.epoch += 1;
        assert!(replay(&f.draw_key, &f.corpus, &later).is_err());
        let other = DrawKey::new(KeyMaterial::from_bytes([0x54; 32]));
        assert!(replay(&other, &f.corpus, &d).is_err());
    }

    /// Weights matter: over many draws the heaviest concepts are picked
    /// first more often than the lightest.
    #[test]
    fn lens_draw_follows_corpus_weights() {
        let f = fixture("weights");
        let h = &f.hosted[0];
        let mut first = std::collections::BTreeMap::new();
        for epoch in 0..600 {
            let d = draw_lens(&f.draw_key, &f.corpus, &inputs(h, epoch)).unwrap();
            *first.entry(d.picks[0]).or_insert(0u32) += 1;
        }
        let count = |w: u32| -> u32 {
            f.corpus
                .entries
                .iter()
                .filter(|e| e.weight == w)
                .map(|e| first.get(&e.concept).copied().unwrap_or(0))
                .sum::<u32>()
                / u32::try_from(f.corpus.entries.iter().filter(|e| e.weight == w).count()).unwrap()
        };
        assert!(count(3) > count(1), "{first:?}");
    }

    #[test]
    fn draw_for_another_cognition_is_refused() {
        let f = fixture("other");
        let d = draw_lens(&f.draw_key, &f.corpus, &inputs(&f.hosted[0], 0)).unwrap();
        let approved: Vec<&Promise> = f.promises.iter().collect();
        assert_eq!(
            generate(&f.doctrine, &f.hosted[1], &approved, &f.corpus, &d),
            Err(plan::GenError::DrawForOther)
        );
    }
}
