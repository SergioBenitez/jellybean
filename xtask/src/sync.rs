use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::fs::File;

use toml_edit::{value, Table, Array};

use crate::{crate_path, expand::PackExpander};

#[derive(Debug)]
struct PackMetdata {
    crate_name: String,
    version: String,
    local_path: String,
    features: Vec<String>,
}

impl PackMetdata {
    fn read() -> io::Result<Vec<Self>> {
        let mut metadata = vec![];
        for pack in PackExpander::pack_dirs()? {
            let pack = pack?;
            let mut cargo_toml = std::fs::read_to_string(pack.path().join("Cargo.toml"))?
                .parse::<toml_edit::Document>()
                .expect("Cargo.toml");

            let local_path = format!("../packs/{}", pack.file_name().to_string_lossy());
            let crate_name = cargo_toml["package"]["name"].as_str().unwrap().to_string();
            let version = cargo_toml["package"]["version"].as_str().unwrap().to_string();
            let mut features = cargo_toml["features"]["default"]
                .as_array_mut()
                .expect("features.default is array")
                .iter()
                .map(|v| v.as_str().expect("feature.default is array of str"))
                .map(|v| v.to_owned())
                .collect::<Vec<_>>();

            features.sort();
            metadata.push(PackMetdata { crate_name, version, local_path, features });
        }

        metadata.sort_by(|a, b| a.crate_name.cmp(&b.crate_name));
        Ok(metadata)
    }

    fn sync_cargo_toml(metadata: &[PackMetdata], toml_path: &Path) -> io::Result<()> {
        let mut manifest = std::fs::read_to_string(toml_path)?
            .parse::<toml_edit::Document>()
            .expect("Cargo.toml");

        manifest["dependencies"]
            .as_table_mut()
            .expect("deps tables")
            .retain(|k, _| !k.starts_with("jellybean-pack"));

        manifest["build-dependencies"]
            .as_table_mut()
            .expect("build-deps tables")
            .retain(|k, _| !k.starts_with("jellybean-pack"));

        let explicit_features = manifest["package"]["metadata"]["features"]
            .as_array().expect("package.metadata.features array")
            .iter()
            .map(|v| v.as_str().expect("package.metadata.feature is [string]"))
            .map(|feature| (feature.to_string(), manifest["features"][feature].clone()))
            .collect::<Vec<_>>();

        manifest["features"].as_table_mut().expect("feature tables").clear();
        for (name, deps) in explicit_features {
            manifest["features"][name] = deps;
        }

        manifest["features"]["default"] = value(Array::new());
        for pack in metadata {
            let mut dep = Table::new();
            dep["path"] = value(&pack.local_path);
            dep["version"] = value(&pack.version);
            dep["default-features"] = value(false);

            let dep_table = dep.into_inline_table();
            manifest["dependencies"][&pack.crate_name] = value(dep_table.clone());
            manifest["build-dependencies"][&pack.crate_name] = value(dep_table);

            for feature in &pack.features {
                let mut feat = Array::new();
                feat.push(format!("{}/{}", pack.crate_name, feature));
                manifest["features"][&feature] = value(feat);
                manifest["features"]["default"].as_array_mut().unwrap().push(feature);
            }
        }

        std::fs::write(toml_path, manifest.to_string())
    }

    fn dep_name(&self) -> String {
        self.crate_name.replace('-', "_")
    }

    fn write_languages_metadata(&self, sink: &mut dyn io::Write) -> io::Result<()> {
        let dep = self.dep_name();

        writeln!(sink, "\t\t&[")?;
        for feature in &self.features {
            writeln!(sink, "\t\t\tLanguageMetadata {{")?;
            writeln!(sink, "\t\t\t\tname: {dep}::{feature}::NAME,")?;
            writeln!(sink, "\t\t\t\tqueries: {dep}::{feature}::QUERIES,")?;
            writeln!(sink, "\t\t\t\tlanguage: {dep}::{feature}::language,")?;
            writeln!(sink, "\t\t\t}},")?;
        }

        writeln!(sink, "\t\t],")
    }

    fn write_metadata_rs(metadata: &[Self], path: &Path) -> io::Result<()> {
        let mut file = BufWriter::new(File::create(path)?);

        writeln!(&mut file, "&[")?;

        for pack in metadata {
            writeln!(&mut file, "\tPackMetdata {{")?;
            writeln!(&mut file, "\t\tdep: {:?},", pack.dep_name())?;
            writeln!(&mut file, "\t\tfeatures: &{:?},", pack.features)?;
            write!(&mut file,   "\t\tlanguages: ")?;
            pack.write_languages_metadata(&mut file)?;
            writeln!(&mut file, "\t}},")?;
        }

        writeln!(&mut file, "]")
    }
}

pub fn main(_: &[&str]) -> io::Result<()> {
    println!(":: syncing lib with packs");

    let metadata = PackMetdata::read()?;
    PackMetdata::sync_cargo_toml(&metadata, crate_path!("..", "lib", "Cargo.toml"))?;
    PackMetdata::write_metadata_rs(&metadata, crate_path!("..", "lib", "metadata.rs"))?;
    Ok(())
}
