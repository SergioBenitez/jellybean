use std::sync::OnceLock;

use tree_sitter_highlight::HighlightConfiguration;

use crate::{Language, Highlighter, EXHAUSTIVE_CAPTURES};

pub struct Dump {
    bytes: &'static [u8],
    cache: &'static OnceLock<HighlightConfiguration>,
}

impl Dump {
    #[inline(always)]
    pub fn force(&self, lang: &'static Language) -> Highlighter {
        let config = self.cache.get_or_init(|| self.decode(lang));
        Highlighter::new(lang, config, &EXHAUSTIVE_CAPTURES[..])
    }

    #[inline(always)]
    pub fn decode(&self, lang: &'static Language) -> HighlightConfiguration {
        let bytes = bincode::deserialize(self.bytes).unwrap();
        HighlightConfiguration::deserialize(bytes, lang.raw()).unwrap()
    }
}

#[inline(always)]
pub fn fetch_config(language: &'static Language) -> HighlightConfiguration {
    DUMPS[language.dump_id].decode(language)
}

#[inline(always)]
pub fn fetch_highlighter(language: &'static Language) -> Highlighter {
    DUMPS[language.dump_id].force(language)
}

macro_rules! define_dump_ids {
    ($($m:ident),*) => {
        define_dump_ids!(0usize, $($m),*);
    };

    ($n:expr, $m:ident $(,)? $($rest:ident),*) => {
        #[allow(non_upper_case_globals)]
        pub const $m: usize = $n;

        define_dump_ids!($n + 1, $($rest),*);
    };

    ($n:expr,) => { }
}

macro_rules! define_dumps {
    ($($m:ident),*) => {
        pub static DUMPS: &[Dump] = &[$(
            Dump {
                bytes: &crate::precached::$m::DUMP,
                cache: {
                    static CACHE: OnceLock<HighlightConfiguration> = OnceLock::new();
                    &CACHE
                }
            }
        ),*];
    };
}

with_all_languages!(define_dump_ids);
with_all_languages!(define_dumps);
