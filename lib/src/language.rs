// use tree_sitter_highlight::HighlightConfiguration;
// use ref_cast::{ref_cast_custom, RefCastCustom};

use uncased::{UncasedStr, AsUncased};
use tree_sitter_highlight::HighlightConfiguration;

use crate::{ALL_LANGUAGES, Highlighter};
use crate::util::cmp_ignore_case_ascii;

/// A materialized tree-sitter language.
pub struct Language {
    /// The name of the language.
    pub(crate) name: &'static str,

    /// The file types recognized by this language, according to the tree-sitter
    /// package.
    ///
    /// This list may be empty. These are typically the file extensions
    /// associated with the language.
    ///
    /// _Example: `["rs"]`_
    ///
    /// See [supported languages](crate#supported-languages) for a full list.
    pub(crate) file_types: &'static [&'static UncasedStr],

    /// The tree-sitter language function.
    pub(crate) language: fn() -> tree_sitter::Language,

    /// A list of tree-sitter queries (name, query data).
    pub(crate) queries: &'static [(&'static str, &'static str)],

    #[cfg(feature = "precached")]
    pub(crate) dump_id: usize,
}

impl Language {
    /// Returns the name of the language.
    ///
    /// This is identical to the feature that needs to be enabled for this
    /// language to be available. For example, the language named `"rust"` is
    /// enabled with `features = ["rust"]`. See [supported languages] for a
    /// complete list of languages, and note that they are all enabled by
    /// default.
    ///
    /// [supported languages]: crate#support-languages
    ///
    /// # Example
    ///
    /// ```rust
    /// use jellybean::Language;
    ///
    /// assert_eq!(Language::rust.name(), "rust");
    /// assert_eq!(Language::cpp.name(), "cpp");
    /// ```
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns an iterator over the pairs of `(query name, query data)`.
    ///
    /// This is the raw tree-sitter query data for the language. It is
    /// guaranteed to contain at least a "highlights" query.
    ///
    /// ```rust
    /// use jellybean::Language;
    ///
    /// assert!(Language::rust.queries().any(|(name, data)| name == "highlights"));
    /// ```
    pub fn queries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.queries.iter().copied()
    }

    /// Returns a sorted slice of the file types recognized by this language.
    ///
    /// # Example
    ///
    /// ```rust
    /// use jellybean::Language;
    ///
    /// let rust_fts = Language::rust.file_types();
    /// assert!(rust_fts.iter().any(|ft| ft == "rs"));
    /// assert!(rust_fts.binary_search(&"rs".into()).is_ok());
    /// ```
    pub fn file_types(&self) -> &[&UncasedStr] {
        &self.file_types
    }

    /// Returns the raw [`tree_sitter::Language`] associated with `self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use jellybean::Language;
    ///
    /// let language = Language::rust.raw();
    /// ```
    pub fn raw(&self) -> tree_sitter::Language {
        (self.language)()
    }

    pub fn find_query(&self, name: &str) -> Option<&str> {
        self.queries.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| *v)
    }

    pub fn highlight_config(&self, highlights: &[&str]) -> HighlightConfiguration {
        let mut config = HighlightConfiguration::new(
            (self.language)(),
            self.name,
            self.find_query("highlights").unwrap_or(""),
            self.find_query("injections").unwrap_or(""),
            self.find_query("locals").unwrap_or(""),
            true,
        ).expect("all queries pre-tested");

        config.configure(highlights);
        config
    }

    pub fn custom_highlighter(
        self: &'static Self,
        captures: &'static [&'static str]
    ) -> Highlighter {
        let config = self.highlight_config(captures);
        Highlighter::new(self, config, captures)
    }

    #[cfg(feature = "precached")]
    pub fn highlighter(self: &'static Self) -> Highlighter {
        crate::dumps::fetch_highlighter(self)
    }
}

macro_rules! define_associated_const {
    ($($m:ident),*) => {
        $(
            #[doc = concat!("The `", stringify!($m), "` tree-sitter language.")]
            ///
            /// This is constructed from the components in
            #[doc = concat!("[`crate::raw::", stringify!($m), "`].")]
            #[allow(non_upper_case_globals)]
            pub const $m: Language = Language {
                name: crate::raw::$m::NAME,
                file_types: unsafe { std::mem::transmute(crate::raw::$m::FILE_TYPES) },
                language: crate::raw::$m::language,
                queries: crate::raw::$m::QUERIES,
                #[cfg(feature = "precached")]
                dump_id: crate::dumps::$m,
            };
        )*
    }
}

impl Language {
    #[inline]
    pub fn find(token: &str) -> Option<&'static Language> {
        Self::find_by_name(token).or_else(|| Self::find_by_file_type(token))
    }

    #[inline]
    pub fn find_by_name(name: &str) -> Option<&'static Language> {
        Self::position_by_name(name).and_then(|i| ALL_LANGUAGES.get(i).copied())
    }

    #[inline]
    pub fn find_by_file_type(file_type: &str) -> Option<&'static Language> {
        Self::position_by_file_type(file_type).and_then(|i| ALL_LANGUAGES.get(i).copied())
    }

    #[inline]
    pub fn position(token: &str) -> Option<usize> {
        Self::position_by_name(token).or_else(|| Self::position_by_file_type(token))
    }

    #[inline]
    pub fn position_by_name(name: &str) -> Option<usize> {
        ALL_LANGUAGES.binary_search_by(|l| cmp_ignore_case_ascii(l.name, name)).ok()
    }

    pub fn position_by_file_type(file_type: &str) -> Option<usize> {
        ALL_LANGUAGES.iter()
            .map(|l| l.file_types)
            .position(|a| a.binary_search(&file_type.as_uncased()).is_ok())
    }

}

impl Language {
    with_all_languages!(define_associated_const);
}

impl std::fmt::Debug for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Language")
            .field("name", &self.name)
            .field("file_types", &self.file_types)
            .field("language", &self.language)
            .field("queries", &self.queries)
            .finish()
    }
}
