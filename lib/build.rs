#[macro_use]
#[path = "../shared/util.rs"]
mod util;

#[path = "../shared/language.rs"]
mod language;

use std::fs;
use std::env::var_os;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde_json::Value;

use language::Language;

#[derive(Debug)]
struct TsMetadata {
    name: String,
    file_types: Vec<String>,
    queries: Vec<(String, PathBuf)>,
    description: Option<String>,
}

const HIGHLIGHT_QUERIES: &[&str] = &["locals", "highlights", "injections"];

impl TsMetadata {
    fn discover_queries(dir: &Path) -> Option<Vec<(String, PathBuf)>> {
        let entries = fs::read_dir(dir.join("queries")).ok()?;
        Some(entries.filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().map_or(false, |e| e == "scm"))
            .map(|path| (path.file_stem().unwrap().to_string_lossy().into_owned(), path))
            .filter(|(name, _)| HIGHLIGHT_QUERIES.contains(&&**name))
            .collect())
    }

    fn extract(root: &Path, package_json: &Path, name: &str) -> Option<TsMetadata> {
        let file = fs::File::open(package_json).ok()?;
        let reader = io::BufReader::new(file);
        let json: Value = serde_json::from_reader(reader).ok()?;

        let ts_data = json.get("tree-sitter")?;
        let ts_map = match ts_data {
            Value::Array(array) => array.iter()
                .find(|v| v.get("path").map_or(false, |v| v == name))
                .or_else(|| array.first())
                .and_then(|v| v.as_object())?,
            Value::Object(map) => map,
            _ => return None,
        };

        let mut file_types = ts_map.get("file-types")
            .and_then(|ft| ft.as_array())
            .iter()
            .flat_map(|array| array.iter())
            .filter_map(|v| v.as_str())
            .map(|s| s.into())
            .collect::<Vec<_>>();

        file_types.sort();
        let file_types = file_types;

        let queries = Self::discover_queries(root)
            .unwrap_or_else(|| {
                HIGHLIGHT_QUERIES.iter()
                    .filter_map(|query| ts_map.get(*query).map(|value| (query, value)))
                    .filter_map(|(name, value)| Some((name, value.as_array()?.get(0)?.as_str()?)))
                    .map(|(name, path)| (name.to_string(), path.into()))
                    .collect()
            });

        let description = json.get("description")
            .and_then(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        Some(TsMetadata { name: name.into(), file_types, queries, description })
    }

    fn read_from(dir: &Path, name: &str) -> Option<TsMetadata> {
        contains(&dir, "package.json")
            .and_then(|path| Self::extract(dir, &path, name))
            .or_else(|| contains(&dir, "../package.json")
                .and_then(|path| Self::extract(dir, &path, name)))
    }

    fn read_from_or_default(dir: &Path, name: &str) -> TsMetadata {
        Self::read_from(dir, name).unwrap_or_else(|| {
            TsMetadata {
                name: name.into(),
                file_types: vec![],
                queries: Self::discover_queries(dir).unwrap_or_default(),
                description: None
            }
        })
    }

    fn write_macro_line(&self, sink: &mut dyn io::Write) -> io::Result<()> {
        let name = &self.name;
        write!(sink, "\t\t{name} tree_sitter_{name} {:?}, [", self.file_types)?;
        for (kind, path) in &self.queries {
            if let Ok(data) = fs::read_to_string(&path) {
                write!(sink, "({:?}, {:?}),", kind, data)?;
            }
        }

        writeln!(sink, "],")
    }

    fn write_table_line(&self, sink: &mut dyn io::Write) -> io::Result<()> {
        let name = &self.name;
        let file_types = self.file_types.join(", ");
        let description = self.description.as_deref().unwrap_or("N/A");
        writeln!(sink, "| [🔗](Language::{name}) `{name}` | {file_types} | {description} |")
    }
}

fn contains(base: impl AsRef<Path>, path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = base.as_ref().join(path.as_ref());
    path.exists().then_some(path)
}

fn build_language(name: &str, src: &Path) {
    let base_config = cc::Build::new()
        .opt_level(3)
        .include(&src)
        .warnings(false)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .clone();

    if let Some(parser) = contains(src, "parser.c") {
        println!("cargo:rerun-if-changed={}", parser.display());
        base_config.clone().file(parser).compile(&format!("{name}-parser"));
    }

    if let Some(scanner) = contains(src, "scanner.c") {
        println!("cargo:rerun-if-changed={}", scanner.display());
        base_config.clone().file(&scanner).compile(&format!("{name}-scanner"));
    } else if let Some(scanner) = contains(src, "scanner.cc") {
        println!("cargo:rerun-if-changed={}", scanner.display());
        let mut cpp_config = base_config.clone();
        if var_os("TARGET").map_or(false, |v| v == "wasm32-wasi") {
            cpp_config.flag_if_supported("-fno-exceptions");
        }

        cpp_config.cpp(true).file(&scanner).compile(&format!("{name}-scanner"));
    }
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", Language::source_path().display());

    let language_source = Language::read_source()?;
    let mut metadata: Vec<TsMetadata> = Language::parse_source_text(&language_source)
        .collect::<Vec<_>>()
        .par_iter()
        .filter(|l| var_os(format!("CARGO_FEATURE_{}", l.name.to_uppercase())).is_some())
        .map(|lang@Language { name, .. }| {
            let ts_lang = format!("tree-sitter-{name}");
            let ts_lang_dir = lang.local_checkout_dir();
            let path_options = [
                ts_lang_dir.join(name).join("src"),
                ts_lang_dir.join(&ts_lang).join("src"),
                ts_lang_dir.join(format!("tree_sitter_{name}")).join("src"),
                ts_lang_dir.join("src"),
            ];

            let src_path = path_options.iter()
                .find(|p| p.exists())
                .expect(&format!("language src for {name} in {path_options:?}"));

            let real_root = src_path.join("../queries")
                .exists()
                .then(|| src_path.parent().unwrap())
                .unwrap_or(&ts_lang_dir);

            build_language(name, &src_path);
            TsMetadata::read_from_or_default(&real_root, name)
        })
        .collect();

    let out_dir = PathBuf::from(var_os("OUT_DIR").unwrap());
    let mut source_rs = fs::File::create(out_dir.join("source.rs"))?;
    let mut table_md = fs::File::create(out_dir.join("language-table.md"))?;

    writeln!(&mut source_rs, r#"macro_rules! tree_sitter_language {{"#)?;
    writeln!(&mut source_rs, r#"    ($cont:path) => ($cont! {{"#)?;

    metadata.sort_by(|a, b| a.name.cmp(&b.name));
    for metadata in &metadata {
        assert!(metadata.queries.iter().find(|(k, _)| k == "highlights").is_some());
        metadata.write_macro_line(&mut source_rs)?;
        metadata.write_table_line(&mut table_md)?;
    }

    writeln!(&mut source_rs, r#"    }})"#)?;
    writeln!(&mut source_rs, r#"}}"#)?;

    Ok(())
}
