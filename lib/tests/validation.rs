use jellybean::{tree_sitter, Language, ALL_LANGUAGES};

#[test]
fn check_api_compat() {
    for language in ALL_LANGUAGES {
        assert!(language.raw().version() >= tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION);
        assert!(language.raw().version() <= tree_sitter::LANGUAGE_VERSION);
    }
}
