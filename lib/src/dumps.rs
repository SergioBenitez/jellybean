use std::sync::OnceLock;

use tree_sitter_highlight::SerializableHighlightConfig;

pub struct Dump {
    bytes: &'static [u8],
    cache: &'static OnceLock<SerializableHighlightConfig>,
}

impl Dump {
    #[inline(always)]
    pub fn force(&self) -> &SerializableHighlightConfig {
        self.cache.get_or_init(|| bincode::deserialize(self.bytes).unwrap())
    }
}

#[inline(always)]
pub fn fetch(id: usize) -> &'static SerializableHighlightConfig {
    DUMPS[id].force()
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
                    static CACHE: OnceLock<SerializableHighlightConfig> = OnceLock::new();
                    &CACHE
                }
            }
        ),*];
    };
}

with_all_languages!(define_dump_ids);
with_all_languages!(define_dumps);
