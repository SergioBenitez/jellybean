// use tree_sitter_highlight::HighlightConfiguration;
// use ref_cast::{ref_cast_custom, RefCastCustom};

use tree_sitter_highlight::HighlightConfiguration;

use crate::Highlighter;
use crate::util::cmp_ignore_case_ascii;

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

macro_rules! define_lang {
    ($($m:ident),*) => {
        $(
            #[allow(non_upper_case_globals)]
            /// Generated from
            #[doc = concat!("[`raw::", stringify!($m), "`](crate::raw::", stringify!($m), ").")]
            pub const $m: Language = Language {
                name: crate::raw::$m::NAME,
                file_types: crate::raw::$m::FILE_TYPES,
                language: crate::raw::$m::language,
                queries: crate::raw::$m::QUERIES,
            };
        )*
    }
}

macro_rules! collect {
    ($($m:ident),*) => {
        &[$(&Language::$m),*]
    }
}

impl Language {
    pub fn raw(&self) -> tree_sitter::Language {
        (self.language)()
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
    pub fn highlighter<'a>(self: &'static Self, captures: &'a [&'a str]) -> Highlighter<'a> {
        Highlighter::new(self, captures)
    }
}

impl Language {
    pub const ALL: &'static [&'static Language] = with_all_languages!(collect);

    #[inline]
    pub fn find(token: &str) -> Option<&'static Language> {
        Self::find_by_name(token).or_else(|| Self::find_by_file_type(token))
    }

    #[inline]
    pub fn find_by_name(name: &str) -> Option<&'static Language> {
        Self::position_by_name(name).and_then(|i| Language::ALL.get(i).copied())
    }

    #[inline]
    pub fn find_by_file_type(file_type: &str) -> Option<&'static Language> {
        Self::position_by_file_type(file_type).and_then(|i| Language::ALL.get(i).copied())
    }

    #[inline]
    pub fn position(token: &str) -> Option<usize> {
        Self::position_by_name(token).or_else(|| Self::position_by_file_type(token))
    }

    #[inline]
    pub fn position_by_name(name: &str) -> Option<usize> {
        Language::ALL.binary_search_by(|l| cmp_ignore_case_ascii(l.name, name)).ok()
    }

    pub fn position_by_file_type(file_type: &str) -> Option<usize> {
        Language::ALL.iter()
            .map(|l| l.file_types)
            .position(|a| a.binary_search_by(|v| cmp_ignore_case_ascii(v, file_type)).is_ok())
    }

    with_all_languages!(define_lang);
}
