use std::{fs, io};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Language<'a> {
    pub name: &'a str,
    pub git_url: &'a str,
    pub branch: Option<&'a str>,
}

impl Language<'_> {
    pub fn source_path() -> &'static Path {
        crate_path!("artifacts", "languages.csv")
    }

    pub fn checkout_dir() -> &'static Path {
        crate_path!("artifacts", "tree-sitter-languages")
    }

    pub fn local_checkout_dir(&self) -> PathBuf {
        Self::checkout_dir().join(self.name)
    }

    pub fn read_source() -> io::Result<String> {
        fs::read_to_string(Self::source_path())
    }

    pub fn parse_source_text(text: &str) -> impl Iterator<Item = Language<'_>> {
        text.lines()
            .filter(|l| !(l.starts_with('#') || l.is_empty()))
            .map(|l| l.split(',').map(|s| s.trim()))
            .map(|mut splits| Language {
                name: splits.next().expect("language name"),
                git_url: splits.next().expect("language git url"),
                branch: splits.next()
            })
    }
}
