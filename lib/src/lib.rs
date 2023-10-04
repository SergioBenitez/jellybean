//! Tree-sitter based syntax highlighting.
//!
//! # Supported Languages
//!
//! | Feature         | File Types | Description                        |
//! |-----------------|------------|------------------------------------|
#![doc = docs!(doc_line)]

// This expands (from build.rs) to the `raw` module, a `docs` macro which calls
// the `doc!()` macro declred here with each language (used above), and a
// `with_all_languages` higher-order macro, used in `language.rs`.
include!(concat!(env!("OUT_DIR"), "/raw.rs"));

mod doc;
mod language;
mod util;
mod highlighter;
mod capture;
mod theme;

pub use tree_sitter;
pub use tree_sitter_highlight;

pub use language::Language;
pub use highlighter::{Highlighter, Highlight};
pub use capture::{EXHAUSTIVE_CAPTURES, COMMON_CAPTURES};
pub use theme::Theme;
