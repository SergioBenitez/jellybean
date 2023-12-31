//! Tree-sitter based syntax highlighting.
//!
//! # Supported Languages
//!
//! | Feature         | File Types | Description                        |
//! |-----------------|------------|------------------------------------|
#![doc = docs!(doc_line)]

#![recursion_limit = "512"]

// This expands (from build.rs) to the `raw` module, a `docs` macro which calls
// the `doc!()` macro declred here with each language (used above), and a
// `with_all_languages` higher-order macro, used in `language.rs`.
include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

mod doc;
mod language;
mod util;
mod highlighter;
mod capture;
mod theme;

#[cfg(feature = "precached")]
pub(crate) mod dumps;

pub use tree_sitter;
pub use tree_sitter_highlight;

pub use language::Language;
pub use highlighter::{Highlighter, Highlight};
pub use theme::Theme;
pub use capture::*;

macro_rules! collect {
    ($($m:ident),*) => (
        /// Foo bar baz.
        pub static ALL_LANGUAGES: &'static [&'static Language] = &[$(&Language::$m),*];
    )
}

with_all_languages!(collect);
