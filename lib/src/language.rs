use tree_sitter_highlight::HighlightConfiguration;
use ref_cast::{ref_cast_custom, RefCastCustom};

use crate::Highlighter;
use crate::util::{cmp_ignore_case_ascii, const_compare};

/// A materialized tree-sitter language.
#[derive(Debug, Copy, Clone)]
pub struct Language {
    /// The name of the language.
    ///
    /// This is identical to the feature that needs to be enabled for this
    /// language to be available.
    ///
    /// _Example: `"rust"` (feature = `rust`)_
    ///
    /// See [supported languages](crate#supported-languages) for a full list.
    ///
    /// [supported languages]: crate#support-languages
    pub name: &'static str,
    /// The file types recognized by this language, according to the tree-sitter
    /// package.
    ///
    /// This list may be empty. These are typically the file extensions
    /// associated with the language.
    ///
    /// _Example: `["rs"]`_
    ///
    /// See [supported languages](crate#supported-languages) for a full list.
    pub file_types: &'static [&'static str],
    /// The tree-sitter language function.
    pub(crate) language: fn() -> tree_sitter::Language,
    /// A list of tree-sitter queries (name, query data).
    pub(crate) queries: &'static [(&'static str, &'static str)],
}

/// A set of languages.
///
/// This is a wrapper around a slice of [`Language`]s that provides methods to
/// query the language set.
#[repr(transparent)]
#[derive(RefCastCustom)]
pub struct LanguageSet([&'static Language]);

impl Language {
    pub fn find_by_name(name: &str) -> Option<&'static Language> {
        LanguageSet::ALL.find_by_name(name)
    }

    pub fn find_by_file_type(file_type: &str) -> Option<&'static Language> {
        LanguageSet::ALL.find_by_file_type(file_type)
    }

    pub fn find_by_any(token: &str) -> Option<&'static Language> {
        LanguageSet::ALL.find_by_any(token)
    }

    pub fn query(self: &'static Self, name: &str) -> Option<&'static str> {
        self.queries.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| *v)
    }

    pub fn highlight_config(self: &'static Self, highlights: &[&str]) -> HighlightConfiguration {
        let mut config = HighlightConfiguration::new(
            (self.language)(),
            self.name,
            self.query("highlights").unwrap_or(""),
            self.query("injections").unwrap_or(""),
            self.query("locals").unwrap_or(""),
            true,
        ).expect("all queries pre-tested");

        config.configure(highlights);
        config
    }

    #[inline]
    pub fn highlighter<'a>(self: &'static Self, highlights: &'a [&'a str]) -> Highlighter<'a> {
        Highlighter::new(self, highlights)
    }
}

impl LanguageSet {
    #[ref_cast_custom]
    const fn _new<'a>(set: &'a [&'static Language]) -> &'a LanguageSet;

    pub const fn new<'a>(set: &'a [&'static Language]) -> &'a LanguageSet {
        let set = Self::_new(set);

        let mut i = 1;
        while i < set.0.len() {
            let (a, b) = (set.0[i - 1], set.0[i]);
            if const_compare(a.name.as_bytes(), b.name.as_bytes()).is_gt() {
                panic!("language set must be sorted by name");
            }

            i += 1;
        }

        set
    }

    pub(crate) fn position_by_name(&self, name: &str) -> Option<usize> {
        self.binary_search_by(|lang| cmp_ignore_case_ascii(lang.name, name)).ok()
    }

    pub(crate) fn position_by_file_type(&self, file_type: &str) -> Option<usize> {
        self.iter()
            .map(|lang| lang.file_types)
            .position(|a| a.binary_search_by(|v| cmp_ignore_case_ascii(v, file_type)).is_ok())
    }

    pub(crate) fn position_by_any(&self, token: &str) -> Option<usize> {
        self.position_by_name(token).or_else(|| self.position_by_file_type(token))
    }

    pub fn find_by_name(&self, name: &str) -> Option<&'static Language> {
        self.position_by_name(name).map(|i| self[i])
    }

    pub fn find_by_file_type(&self, file_type: &str) -> Option<&'static Language> {
        self.position_by_file_type(file_type).map(|i| self[i])
    }

    pub fn find_by_any(&self, token: &str) -> Option<&'static Language> {
        self.position_by_any(token).map(|i| self[i])
    }

    // #[inline]
    // pub fn to_highlight_set(&self, recognize: &[&str]) -> HighlighterSet {
    //     HighlighterSet::from_language_set(self, recognize)
    // }
}

impl std::ops::Deref for LanguageSet {
    type Target = [&'static Language];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
