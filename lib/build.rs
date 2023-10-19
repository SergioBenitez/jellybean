use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::io::{self, BufWriter, Write};

include!("src/capture.rs");

// Written by `xtask sync`.
const PACKS: &[PackMetdata] = include!("metadata.rs");

struct LanguageMetadata {
    name: &'static str,
    queries: &'static [(&'static str, &'static str)],
    language: fn() -> tree_sitter::Language,
}

struct PackMetdata {
    dep: &'static str,
    features: &'static [&'static str],
    languages: &'static [LanguageMetadata],
}

#[cfg(feature = "precached")]
mod precached {
    use super::*;
    use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
    use tree_sitter_highlight::HighlightConfiguration;

    impl LanguageMetadata {
        fn query(&self, name: &str) -> Option<&'static str> {
            self.queries.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| *v)
        }

        pub fn highlight_config(&self) -> HighlightConfiguration {
            let config = HighlightConfiguration::new(
                (self.language)(),
                self.name,
                self.query("highlights").unwrap_or(""),
                self.query("injections").unwrap_or(""),
                self.query("locals").unwrap_or(""),
                true,
            );

            if let Err(e) = config {
                eprintln!("HighlightConfig failure for: {}\n{e}", self.name);
                panic!("queries failed");
            }

            let mut config = config.unwrap();
            config.configure(EXHAUSTIVE_CAPTURES);
            config
        }
    }

    pub fn write_serialized_module(sink: &mut dyn io::Write) -> io::Result<()> {
        let dumps = PACKS.par_iter()
            .flat_map(|p| p.languages.par_iter())
            .filter(|l| crate_feature_active(l.name))
            .map(|l| (l.name, l.highlight_config().serializable().expect(l.name)))
            .map(|(name, hl)| (name, bincode::serialize(&hl).expect(name)))
            .collect::<Vec<_>>();

        writeln!(sink, r#"#[cfg(feature = "precached")]"#)?;
        writeln!(sink, "mod precached {{")?;
        for (lang, dump) in dumps {
            writeln!(sink, "pub mod {lang} {{")?;
            writeln!(sink, "pub const DUMP: &'static [u8] = &{dump:?};")?;
            writeln!(sink, "}}")?;
        }

        writeln!(sink, "}}")
    }
}

fn crate_feature_active(feat: &str) -> bool {
    env::var_os(format!("CARGO_FEATURE_{}", feat.to_uppercase())).is_some()
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=metadata.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut sink = BufWriter::new(File::create(out_dir.join("codegen.rs"))?);

    // Generate a global import.
    writeln!(&mut sink, "/// Raw tree-sitter languages.")?;
    writeln!(&mut sink, "#[allow(ambiguous_glob_reexports)]")?;
    writeln!(&mut sink, "pub mod raw {{")?;
    PACKS.iter().try_for_each(|p| writeln!(&mut sink, "pub use {}::*;", p.dep))?;
    writeln!(&mut sink, "}}")?;

    // Generate a global import.
    #[cfg(feature = "precached")]
    precached::write_serialized_module(&mut sink)?;

    // Generate the docs macro.
    writeln!(&mut sink, "#[doc(hidden)]")?;
    writeln!(&mut sink, "#[macro_export]")?;
    writeln!(&mut sink, "macro_rules! docs {{")?;
    writeln!(&mut sink, "    ($doc:ident) => {{")?;
    writeln!(&mut sink, "        concat!(")?;
    PACKS.iter().try_for_each(|p| writeln!(&mut sink, "\t\t{}::with_languages!($doc),", p.dep))?;
    writeln!(&mut sink, "        )")?;
    writeln!(&mut sink, "    }}")?;
    writeln!(&mut sink, "}}")?;

    // Generate the languages macro.
    let all_langs = PACKS.iter()
        .flat_map(|p| p.features.iter().copied())
        .filter(|f| crate_feature_active(f))
        .collect::<Vec<_>>()
        .join(",");

    writeln!(&mut sink, "#[doc(hidden)]")?;
    writeln!(&mut sink, "#[macro_export]")?;
    writeln!(&mut sink, "macro_rules! with_all_languages {{")?;
    writeln!(&mut sink, "    ($m:ident) => {{")?;
    writeln!(&mut sink, "        $m! {{ {all_langs} }}")?;
    writeln!(&mut sink, "    }}")?;
    writeln!(&mut sink, "}}")?;

    Ok(())
}
