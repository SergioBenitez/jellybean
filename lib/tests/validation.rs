use jellybean::{tree_sitter, ALL_LANGUAGES, EXHAUSTIVE_CAPTURES};

#[test]
fn check_api_compat() {
    for language in ALL_LANGUAGES {
        assert!(language.raw().version() >= tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION);
        assert!(language.raw().version() <= tree_sitter::LANGUAGE_VERSION);
    }
}

#[test]
fn check_hl_creation() {
    for language in ALL_LANGUAGES {
        let hl = language.custom_highlighter(EXHAUSTIVE_CAPTURES);
        assert_eq!(hl.language().name(), language.name())
    }
}
