//! Tree-sitter based syntax highlighting.
//!
//! # Usage
//!
//! Depend on the library and enable the languages (or the `all-languages`
//! feature for all of them) you want to support:
//!
//! ```toml
//! [dependencies.jellybean]
//! version = "1"
//! features = ["c", "rust", "bash", "toml"]
//! ```
//!
//! Highlight some code:
//!
//! ```rust
//! use jellybean::{Language, Highlighter, Highlight};
//!
//! const SOURCE: &str = r#"fn main() {
//!    println!("Hello, world!");
//! }"#;
//!
//! let config = Language::rust.highlight_config(jellybean::BASE_HIGHLIGHTS).unwrap();
//! for event in Highlighter::new(&config).highlight(SOURCE) {
//!     match event.unwrap() {
//!         Highlight::Start { highlight, .. } => print!("<hl name={highlight:?}>"),
//!         Highlight::Source { text, .. } => print!("{}", text.replace('\n', "<br />\n")),
//!         Highlight::End => print!("</hl>"),
//!     }
//! }
//! ```
//!
//! ```html
//! <hl name="function.macro">fn</hl> <hl name="function.method">main</hl><hl name="punctuation.bracket">(</hl><hl name="punctuation.bracket">)</hl> <hl name="punctuation.bracket">{</hl><br />
//!     <hl name="function">println</hl><hl name="function">!</hl><hl name="punctuation.bracket">(</hl><hl name="variable.parameter">"Hello, world!"</hl><hl name="punctuation.bracket">)</hl><hl name="punctuation.delimiter">;</hl><br />
//! <hl name="punctuation.bracket">}</hl>
//! ```
//!
//! # Supported Languages
//!
//! | Feature         | File Types | Description                        |
//! |-----------------|------------|------------------------------------|
//! | `all-languages` |            | enables support for every language |
#![doc = include_str!(concat!(env!("OUT_DIR"), "/language-table.md"))]

mod language;
mod highlight_set;
mod highlighter;
mod languages;
mod util;

pub use tree_sitter;
pub use tree_sitter_highlight;

pub use language::{Language, LanguageSet};
pub use highlighter::{Highlighter, Highlight};
// pub use highlight_set::HighlighterSet;

pub type Result<T, E = tree_sitter_highlight::Error> = std::result::Result<T, E>;

pub const BASE_HIGHLIGHTS: [&str; 24] = [
    "attribute",
    "label",
    "constant",
    "function.builtin",
    "function.macro",
    "function",
    "keyword",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "tag",
    "escape",
    "type",
    "type.builtin",
    "constructor",
    "variable",
    "variable.builtin",
    "variable.parameter",
    "comment",
    "repeat",
];
