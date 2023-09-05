use std::fs::{File, DirEntry};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{PathBuf, Path};

use rayon::prelude::*;
use serde_json::{Map, Value};

const HIGHLIGHT_QUERIES: &[&str] = &["locals", "highlights", "injections"];
const LANGUAGE_PACK: &str = "pack.tar.zst";
const LANGUAGE_DIR: &str = "languages";

#[derive(Debug, Default)]
struct TsMetadata {
    name: String,
    enabled: bool,
    src_dir: PathBuf,
    file_types: Vec<String>,
    queries: Vec<(String, PathBuf)>,
    description: String,
}

impl TsMetadata {
    fn read(language: &DirEntry) -> TsMetadata {
        let path = language.path();
        let name = language.file_name().to_string_lossy().to_string();

        let enabled = true;
        if std::env::var_os(format!("CARGO_FEATURE_{}", name.to_uppercase())).is_none() {
            return TsMetadata { name, enabled: false, ..Default::default() };
        }

        let root_candidates = [
            path.join(&name),
            path.join(format!("tree-sitter-{name}")),
            path.join(format!("tree_sitter_{name}")),
            path.clone(),
        ];

        let src_dir = root_candidates.iter()
            .map(|p| p.join("src"))
            .find(|p| p.exists())
            .expect(&format!("{name}: missing src ({root_candidates:?})"));

        let (description, ts_json) = root_candidates.iter()
            .map(|root| root.join("package.json"))
            .filter_map(|path| Self::parse_package_json(&name, &path))
            .next()
            .expect(&format!("{name}: missing package.json ({root_candidates:?})"));

        let mut file_types = ts_json.get("file-types")
            .and_then(|ft| ft.as_array())
            .into_iter()
            .flat_map(|array| array.iter())
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        file_types.sort();

        let scope = ts_json.get("scope")
            .and_then(|scope| scope.as_str())
            .and_then(|scope| scope.rsplit('.').next())
            .map(|scope| path.parent().expect("language parent dir").join(scope))
            .unwrap_or(PathBuf::new());

        let queries = root_candidates.iter()
            .map(|root| root.join("queries"))
            .find_map(|path| Self::discover_queries(&path))
            .unwrap_or_else(|| {
                HIGHLIGHT_QUERIES.iter()
                    .filter_map(|query| ts_json.get(*query).map(|value| (query, value)))
                    .filter_map(|(name, v)| Some((name, v.as_array()?.first()?.as_str()?)))
                    .map(|(name, path)| (name.to_string(), scope.join(path)))
                    .collect()
            });

        TsMetadata { name, enabled, src_dir, file_types, queries, description, }
    }

    fn parse_package_json(name: &str, path: &Path) -> Option<(String, Map<String, Value>)> {
        let reader = io::BufReader::new(File::open(path).ok()?);
        let mut json: Value = serde_json::from_reader(reader).ok()?;

        let _ = json.get("name")?;
        let description = json.get("description")
            .and_then(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let map = match json.get_mut("tree-sitter").map(std::mem::take) {
            Some(Value::Object(map)) => Some(map),
            Some(Value::Array(mut array)) => {
                let inner = array.iter_mut()
                    .find(|v| v.get("path").map_or(false, |v| v == name));

                let item = match inner {
                    Some(item) => item,
                    None => array.first_mut()?,
                };

                item.as_object_mut().map(std::mem::take)
            },
            _ => None
        };

        Some((description, map.unwrap_or_default()))
    }

    fn discover_queries(path: &Path) -> Option<Vec<(String, PathBuf)>> {
        let entries = path.read_dir().ok()?;
        let queries = entries.filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().map_or(false, |e| e == "scm"))
            .map(|path| (path.file_stem().unwrap().to_string_lossy().into_owned(), path))
            .filter(|(name, _)| HIGHLIGHT_QUERIES.contains(&&**name))
            .collect();

        Some(queries)
    }

    fn assert_queries(&self) {
        if !self.queries.iter().any(|(k, _)| k == "highlights") {
            panic!("{} is missing highlights query", self.name);
        }

        for (query, path) in &self.queries {
            if !path.exists() {
                panic!("{} query {query:?} ({path:?}) not found", self.name);
            }
        }
    }

    fn write_macro_line(&self, sink: &mut dyn io::Write) -> io::Result<()> {
        let TsMetadata { name, file_types, description, .. } = self;
        write!(sink, "\t\t{name}, {name:?}, {description:?}, {file_types:?}")
    }

    fn write_docs_line(&self, sink: &mut dyn io::Write) -> io::Result<()> {
        let TsMetadata { name, file_types, description, .. } = self;
        writeln!(sink, "| [`{name}`] | {} | {description} |", file_types.join(", "))
    }

    fn write_module_line(&self, sink: &mut dyn io::Write) -> io::Result<()> {
        let TsMetadata { name, file_types, queries, description, .. } = self;

        let query_keys = queries.iter().map(|k| &k.0).collect::<Vec<_>>();
        let expanded_queries: Vec<_> = queries.iter()
            .map(|(name, path)| (name, std::fs::read_to_string(path).expect("query I/O")))
            .collect();

        writeln!(sink, r#"
            /// The `{name}` tree-sitter language.
            pub mod {name} {{
                /// The stringified language name: `{name:?}`.
                pub const NAME: &'static str = {name:?};

                /// A description of the tree-sitter language. May be empty.
                pub const DESCRIPTION: &'static str = {description:?};

                /// The file types reported as supported by the language.
                ///
                /// This is a slice of file types (i.e, file extensions) that
                /// identify this language.
                pub const FILE_TYPES: &'static [&'static str] = &{file_types:?};

                /// The highlighting queries.
                ///
                /// The slice contains (key, value) pairs where the key is the
                /// name of the query and the value is the query itself. This
                /// language contains the following query keys: {query_keys:?}.
                pub const QUERIES: &'static [(&'static str, &'static str)] = &{expanded_queries:?};

                /// The tree-sitter language structure.
                pub fn language() -> tree_sitter::Language {{
                    extern "C" {{ fn tree_sitter_{name}() -> tree_sitter::Language; }}
                    unsafe {{ tree_sitter_{name}() }}
                }}
            }}
        "#)
    }

    fn compile(&self) {
        fn entry(base: impl AsRef<Path>, path: impl AsRef<Path>) -> Option<PathBuf> {
            let path = base.as_ref().join(path.as_ref());
            path.exists().then_some(path)
        }

        let name = &self.name;
        let base_config = cc::Build::new()
            .opt_level(3)
            .include(&self.src_dir)
            .warnings(false)
            .flag_if_supported("-Wno-c++11-extensions")
            .flag_if_supported("-Wno-null-character")
            .flag_if_supported("-Wno-macro-redefined")
            .flag_if_supported("-Wno-unused-parameter")
            .flag_if_supported("-Wno-unused-but-set-variable")
            .flag_if_supported("-Wno-trigraphs")
            .clone();

        if let Some(parser) = entry(&self.src_dir, "parser.c") {
            println!("cargo:rerun-if-changed={}", parser.display());
            base_config.clone().file(parser).compile(&format!("{name}-parser"));
        }

        if let Some(scanner) = entry(&self.src_dir, "scanner.c") {
            println!("cargo:rerun-if-changed={}", scanner.display());
            base_config.clone().file(&scanner).compile(&format!("{name}-scanner"));
        } else if let Some(scanner) = entry(&self.src_dir, "scanner.cc") {
            println!("cargo:rerun-if-changed={}", scanner.display());
            let mut cpp_config = base_config.clone();
            if std::env::var_os("TARGET").map_or(false, |v| v == "wasm32-wasi") {
                cpp_config.flag_if_supported("-fno-exceptions");
            }

            cpp_config.cpp(true).file(&scanner).compile(&format!("{name}-scanner"));
        }
    }
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", LANGUAGE_PACK);

    // Get `OUT_DIR` from Cargo.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Decompress the language pack.
    let tar_path = out_dir.join(LANGUAGE_PACK).with_extension("tar");
    let reader = BufReader::new(File::open(LANGUAGE_PACK)?);
    let writer = BufWriter::new(File::create(&tar_path)?);
    zstd::stream::copy_decode(reader, writer)?;

    // Untar the decompressed pack.
    let languages_dir = out_dir.join(LANGUAGE_DIR);
    let mut tarball = tar::Archive::new(BufReader::new(File::open(&tar_path)?));
    tarball.unpack(&languages_dir)?;

    // Collect a vector of enabled languages and their metadata.
    let mut metadata = languages_dir.read_dir()?
        .collect::<Result<Vec<_>, _>>()?
        .par_iter()
        .map(TsMetadata::read)
        .filter(|metadata| metadata.enabled)
        .inspect(|metadata| metadata.assert_queries())
        .collect::<Vec<_>>();

    // Sort, then compile each language in parallel.
    metadata.sort_by(|a, b| a.name.cmp(&b.name));
    metadata.par_iter().for_each(|lang| lang.compile());

    // Write out the second-order macro source. This also serves as a doc-test
    // to ensure that each language is compiled correctly.
    let mut macro_rs = BufWriter::new(File::create(out_dir.join("macro.rs"))?);
    writeln!(&mut macro_rs, "
    /// A second-order macro invoking a macro with the languages in this pack.
    ///
    /// That macro is invoked once. It is invoked with the following syntax:
    ///
    /// ```ignore
    /// $($module:ident, $name:literal, $description:literal, [$($file_type:literal),*])*
    /// ```
    ///
    /// * `$module` - the identifier for the language module in this pack
    /// * `$name` - the string literal for the language name
    /// * `$description` - the string literal for the lanugage description
    /// * `$file_type` - a string literal for a recognized language file type
    ///
    /// The languages are guaranteed to be lexicographically sorted by name.
    ///
    /// Example:
    ///
    /// ```rust
    /// use {krate}::with_languages;
    ///
    /// macro_rules! all_languages {{
    ///     ($($m:ident, $name:literal, $desc:literal, [$($ft:literal),*])*) => {{
    ///         &[$({krate}::$m::language()),*]
    ///     }}
    /// }}
    ///
    /// let all_languages: &[tree_sitter::Language] = with_languages!(all_languages);
    /// ```
    /// ", krate = std::env::var("CARGO_PKG_NAME").unwrap().replace("-", "_"))?;
    writeln!(&mut macro_rs, "#[macro_export]")?;
    writeln!(&mut macro_rs, "macro_rules! with_languages {{")?;
    writeln!(&mut macro_rs, r#"    ($cont:ident) => {{ $cont! {{"#)?;
    metadata.iter().try_for_each(|m| m.write_macro_line(&mut macro_rs))?;
    writeln!(&mut macro_rs, r#"    }} }}"#)?;
    writeln!(&mut macro_rs, r#"}}"#)?;

    // Generate all of the language modules.
    let mut modules_rs = File::create(out_dir.join("modules.rs"))?;
    metadata.iter().try_for_each(|m| m.write_module_line(&mut modules_rs))?;

    // Generate the markdown to document each language.
    let mut docs_md = File::create(out_dir.join("docs.md"))?;
    metadata.iter().try_for_each(|m| m.write_docs_line(&mut docs_md))?;

    Ok(())
}
