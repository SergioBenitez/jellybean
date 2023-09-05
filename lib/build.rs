use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::io::{self, BufWriter, Write};

struct PackMetdata {
    dep: &'static str,
    features: &'static [&'static str],
}

fn main() -> io::Result<()> {
    const METADATA: &[PackMetdata] = include!("metadata.rs");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut sink = BufWriter::new(File::create(out_dir.join("raw.rs"))?);

    // Generate a global import.
    writeln!(&mut sink, "/// Raw tree-sitter languages.")?;
    writeln!(&mut sink, "#[allow(ambiguous_glob_reexports)]")?;
    writeln!(&mut sink, "pub mod raw {{")?;
    METADATA.iter().try_for_each(|m| writeln!(&mut sink, "pub use {}::*;", m.dep))?;
    writeln!(&mut sink, "}}")?;

    // Generate the docs macro.
    writeln!(&mut sink, "#[doc(hidden)]")?;
    writeln!(&mut sink, "#[macro_export]")?;
    writeln!(&mut sink, "macro_rules! docs {{")?;
    writeln!(&mut sink, "    ($doc:ident) => {{")?;
    writeln!(&mut sink, "        concat!(")?;
    METADATA.iter().try_for_each(|m| writeln!(&mut sink, "\t\t{}::with_languages!($doc),", m.dep))?;
    writeln!(&mut sink, "        )")?;
    writeln!(&mut sink, "    }}")?;
    writeln!(&mut sink, "}}")?;

    // Generate the languages macro.
    let all_langs = METADATA.iter()
        .flat_map(|m| m.features.iter())
        .copied()
        .filter(|f| env::var_os(format!("CARGO_FEATURE_{}", f.to_uppercase())).is_some())
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
