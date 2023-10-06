use std::{fs, io};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use std::sync::OnceLock;

use indicatif::{ProgressBar, ProgressStyle};

use crate::util::{Semaphore, visible, flag, verbose};
use crate::{crate_path, cmd, vprintln};

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
        let branch = self.branch.unwrap_or("default");
        let verbosity = if verbose() { "--progress" } else { "-q" };
        let verbosity2 = if verbose() { "" } else { "-q" };
        if already_exists {
            if update {
                vprintln!("= {} ({}:{branch})", self.name, self.git_url);
                cmd!(&lang_dir => "git", "reset", "--hard", verbosity2, "HEAD")?;
                cmd!(&lang_dir => "git", "pull", "--rebase", "--depth=1", verbosity)?;
            }
        } else {
            vprintln!("+ {} ({}:{branch})", self.name, self.git_url);
            let depth = match (&self.branch, &self.main) {
                (None, None) => &["--depth=1"][..],
                _ => &["--depth=1", "--no-single-branch"][..],
            };

            cmd! {
                "git", "clone", #depth, verbosity, self.git_url, &lang_dir
            }?;

            if let Some(branch) = self.branch {
                vprintln!("= switching to {branch}");
                cmd!(&lang_dir => "git", "fetch", verbosity, "--depth=1", "origin", branch)?;
                cmd!(&lang_dir => "git", "switch", verbosity, branch)?;
            }
        }

        if let Some(main) = self.main {
            if !already_exists || update {
                vprintln!("= {} package.json from {main} branch", self.name);
                cmd!(&lang_dir => "git", "fetch", verbosity, "--depth=1", "origin", main)?;
                cmd!(&lang_dir => "git", "checkout", verbosity, main, "package.json")?;
            }
        }

        // Remove any `Cargo.toml` so `cargo publish` doesn't ignore the dir.
        let walker = walkdir::WalkDir::new(&lang_dir).max_depth(3).into_iter();
        for entry in walker.filter_entry(visible) {
            let entry = entry?;
            if FORBIDDEN_FILES.iter().any(|&x| x == entry.file_name()) {
                vprintln!("- removing {}", entry.path().display());
                std::fs::remove_file(entry.path())?;
            }
        }

        Ok(())
    }

    pub fn fetch_and_sync_all(update: bool) -> io::Result<()> {
        // Remove any language not in the source file.
        let declared_languages = TsLanguage::iter().collect::<Vec<_>>();
        if Self::checkout_container().exists() {
            let declared_language_paths = declared_languages.iter()
                .map(|lang| lang.checkout_path())
                .collect::<HashSet<_>>();

            for entry in Self::checkout_container().read_dir()? {
                let entry_path = entry?.path();
                if !declared_language_paths.contains(&entry_path) {
                    vprintln!("- removing {}", entry_path.display());
                    std::fs::remove_dir_all(entry_path)?;
                }
            }
        }

        let template = "{spinner}{msg} {prefix}: {wide_bar} {pos}/{len}";
        let progress = ProgressBar::new(declared_languages.len() as u64)
            .with_style(ProgressStyle::with_template(template).unwrap())
            .with_prefix("progress");

        // Fetch/update all languages in the source file.
        let mut threads = vec![];
        let gate = Semaphore::new(32);
        for lang in declared_languages {
            let token = gate.take();
            let progress = progress.clone();
            threads.push(std::thread::spawn(move || {
                lang.fetch(update).expect(lang.name);
                drop(token);
                progress.inc(1);
            }));
        }

        threads.into_iter().for_each(|t| t.join().expect("fetch panicked"));
        progress.finish_with_message("✓");
        Ok(())
    }
}

pub fn main(args: &[&str]) -> io::Result<()> {
    let update = flag(args, "u");
    println!(":: fetching languages (updating? {update})");
    TsLanguage::fetch_and_sync_all(update)
}
