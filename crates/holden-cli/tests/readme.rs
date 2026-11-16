//! Public README contracts that do not require running the macOS quickstart.

#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    /// Slice-12b P4: the one real-key migration command is documented as an
    /// owner-only terminal action that agents never execute.
    /// Fail-without mutant: remove the owner-only key migration section.
    #[test]
    fn readme_names_the_owner_migration_command() {
        let readme = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md"),
        )
        .unwrap();
        let section = readme
            .split_once("## Owner-only key migration\n")
            .unwrap()
            .1
            .split_once("\n## ")
            .unwrap()
            .0;
        assert!(section.contains("**OWNER ONLY**"));
        assert!(section.contains("from the owner's terminal"));
        assert_eq!(
            section
                .matches("```sh\nholden migrate-owner-key\n```")
                .count(),
            1,
            "README must document exactly one migration command"
        );
        assert!(section.contains("Agents never run `holden migrate-owner-key`"));
    }
}
