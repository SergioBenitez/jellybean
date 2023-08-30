use std::io::{self, Error, ErrorKind::Other};

use toml_edit::{value, Array, Value};

use crate::language::Language;

pub fn sync<'a, I>(languages: I) -> io::Result<()>
    where I: IntoIterator<Item = Language<'a>>
{
    let lib_cargo_path = workspace!("lib", "Cargo.toml");
    let lib_cargo_toml = std::fs::read_to_string(&lib_cargo_path)?;
    let mut lib_cargo_toml: toml_edit::Document = lib_cargo_toml.parse()
        .map_err(|e| Error::new(Other, format!("lib/Cargo.toml error: {}", e)))?;

    let lib_features = lib_cargo_toml.get_mut("features")
        .and_then(|item| item.as_table_mut())
        .expect("have features table");

    lib_features.clear();
    for lang in languages {
        lib_features.insert(lang.name, value(Array::new()));
    }

    let all_languages = lib_features.iter()
        .map(|(name, _)| name)
        .collect::<Value>();

    lib_features.insert("all-languages", value(all_languages));

    std::fs::write(&lib_cargo_path, lib_cargo_toml.to_string())
}

pub fn main(_: &[&str]) -> io::Result<()> {
    println!(":: syncing lib/Cargo.toml features");
    let language_source = Language::read_source()?;
    sync(Language::parse_source_text(&language_source))
}
