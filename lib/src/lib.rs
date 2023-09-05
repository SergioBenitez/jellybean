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

pub use tree_sitter;
pub use tree_sitter_highlight;

pub use language::Language;
pub use highlighter::{Highlighter, Highlight};
pub use capture::{EXHAUSTIVE_CAPTURES, COMMON_CAPTURES};
pub use theme::Theme;

mod theme {
    use std::borrow::Cow;

    use rustc_hash::FxHashMap;

    // NOTE: If we make this an enum with static and dynamic variants, we can
    // make it really convenient to use and maybe even pass it in to
    // `Highlighter` and it'll return the thing it finds (or `None`).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Theme<T> {
        map: FxHashMap<Cow<'static, str>, T>
    }

    impl<S, T> FromIterator<(S, T)> for Theme<T>
        where S: Into<Cow<'static, str>>
    {
        fn from_iter<I: IntoIterator<Item = (S, T)>>(iter: I) -> Self {
            let map = iter.into_iter()
                .map(|(k, v)| (k.into(), v))
                .collect();

            Self { map }
        }
    }

    impl<T> Theme<T> {
        pub fn find(&self, capture: &str) -> Option<&T> {
            let mut candidate = capture;
            loop {
                if capture.is_empty() {
                    return None;
                }

                if let Some(value) = self.map.get(candidate) {
                    return Some(value);
                }

                candidate = &candidate[..candidate.rfind('.')?];
            }
        }
    }
}
