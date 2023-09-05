use std::{fs, io};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::util::{Semaphore, visible, flag};
use crate::{crate_path, cmd};

/// Metadata that defines the source location of a tree-sitter language parser.
#[derive(Debug)]
pub struct TsLanguage {
    pub name: &'static str,
    pub git_url: &'static str,
    pub branch: Option<&'static str>,
    pub main: Option<&'static str>,
}

impl TsLanguage {
    pub fn checkout_container() -> &'static Path {
        crate_path!("artifacts", "tree-sitter-language")
    }

    pub fn source_file() -> &'static Path {
        crate_path!("languages.csv")
    }

    pub fn checkout_path(&self) -> PathBuf {
        Self::checkout_container().join(self.name)
    }

    pub fn iter() -> impl Iterator<Item = TsLanguage> {
        static LANGUAGE_SOURCE: OnceLock<String> = OnceLock::new();

        let source = LANGUAGE_SOURCE.get_or_init(|| {
            fs::read_to_string(Self::source_file()).expect("failed to read languages.csv")
        });

        source.lines()
            .filter(|l| !(l.starts_with('#') || l.is_empty()))
            .map(|l| l.split(',').map(|s| s.trim()))
            .map(|mut splits| TsLanguage {
                name: splits.next().expect("language name"),
                git_url: splits.next().expect("language git url"),
                branch: splits.next(),
                main: splits.next(),
            })
    }

    // Clones the source `self` into `self.checkout_path()`.
    pub fn fetch(&self, update: bool) -> io::Result<()> {
        const FORBIDDEN_FILES: &[&str] = &["Cargo.toml", "build.rs"];

        let lang_dir = self.checkout_path();
        let already_exists = lang_dir.exists();
        if already_exists {
            if update {
                // println!("= {} ({}:{})",
                //     self.name, self.git_url, self.branch.unwrap_or("default"));
                cmd!(&lang_dir => "git", "pull", "-q", "--depth=1")?;
            }
        } else {
            println!("+ {} ({}:{})", self.name, self.git_url, self.branch.unwrap_or("default"));
            cmd! {
                "git", "clone", "--depth=1", "--no-single-branch",
                self.git_url,
                &lang_dir
            }?;

            if let Some(branch) = self.branch {
                println!("= switching to {branch}");
                cmd!(&lang_dir => "git", "checkout", branch)?;
            }
        }

        if let Some(main) = self.main {
            if !already_exists || update {
                // println!("= {} package.json from {main} branch", self.name);
                cmd!(&lang_dir => "git", "checkout", "-q", main, "package.json")?;
            }
        }

        // Remove any `Cargo.toml` so `cargo publish` doesn't ignore the dir.
        let walker = walkdir::WalkDir::new(&lang_dir).max_depth(3).into_iter();
        for entry in walker.filter_entry(visible) {
            let entry = entry?;
            if FORBIDDEN_FILES.iter().any(|&x| x == entry.file_name()) {
                println!("- removing {}", entry.path().display());
                std::fs::remove_file(entry.path())?;
            }
        }

        Ok(())
    }

    pub fn fetch_and_sync_all(update: bool) -> io::Result<()> {
        // Remove any language not in the source file.
        if Self::checkout_container().exists() {
            let declared_language_paths = TsLanguage::iter()
                .map(|lang| lang.checkout_path())
                .collect::<HashSet<_>>();

            for entry in Self::checkout_container().read_dir()? {
                let entry_path = entry?.path();
                if !declared_language_paths.contains(&entry_path) {
                    println!("- removing {}", entry_path.display());
                    std::fs::remove_dir_all(entry_path)?;
                }
            }
        }

        // Fetch/update all languages in the source file.
        let mut threads = vec![];
        let gate = Semaphore::new(32);
        for lang in TsLanguage::iter() {
            let token = gate.take();
            threads.push(std::thread::spawn(move || {
                let _token = token;
                lang.fetch(update).expect(lang.name)
            }));
        }

        threads.into_iter().for_each(|t| t.join().expect("fetch panicked"));
        Ok(())
    }
}

pub fn main(args: &[&str]) -> io::Result<()> {
    let update = flag(args, "u");
    println!(":: fetching languages (updating? {update})");
    TsLanguage::fetch_and_sync_all(update)
}
