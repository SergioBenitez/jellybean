use jellybean::{tree_sitter, Language};

#[test]
fn check_api_compat() {
    for language in Language::ALL {
        assert!(language.raw().version() >= tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION);
        assert!(language.raw().version() <= tree_sitter::LANGUAGE_VERSION);
    }
}