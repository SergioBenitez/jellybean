use crate::{LanguageSet, Language};

include!(concat!(env!("OUT_DIR"), "/source.rs"));

macro_rules! define_language_defs {
    ($($name:ident $lang:ident $file_types:expr, $queries:expr),* $(,)?) => (
        $(
            fn $lang() -> tree_sitter::Language {
                extern "C" { fn $lang() -> tree_sitter::Language; }
                unsafe { $lang() }
            }

            #[allow(non_upper_case_globals)]
            pub const $name: $crate::Language = $crate::Language {
                language: Self::$lang,
                name: stringify!($name),
                file_types: & $file_types,
                queries: & $queries,
            };
        )*
    );
}

macro_rules! define_complete_set {
    ($($name:ident $lang:ident $file_types:expr, $queries:expr),* $(,)?) => (
        pub const ALL: &'static LanguageSet = LanguageSet::new(&[
            $(&$crate::Language::$name),*
        ]);
    );
}

impl Language {
    tree_sitter_language!(define_language_defs);
}

impl LanguageSet {
    tree_sitter_language!(define_complete_set);
}
