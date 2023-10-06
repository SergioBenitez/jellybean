use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::collections::HashSet;
use std::time::SystemTime;

use globset::GlobSet;
use walkdir::{WalkDir, DirEntry};
use toml_edit::{value, Document, Array};

use crate::package::PackBuilder;
use crate::util::{globset, visible, flag};
use crate::{crate_path, err_exit};

pub struct PackExpander {
    name: String,
    tarball: PathBuf,
    zball: PathBuf,
}

impl PackExpander {
    pub fn template_dir() -> &'static Path {
        crate_path!("..", "packs", "_template")
    }

    pub fn pack_dirs() -> io::Result<impl Iterator<Item = io::Result<std::fs::DirEntry>>> {
        let entries = crate_path!("..", "packs")
            .read_dir()?
            .filter(|e| e.as_ref().map_or(false, |e| e.file_name() != "_template"));

        Ok(entries)
    }

    pub fn target_dir(&self) -> PathBuf {
        crate_path!("..", "packs").join(&self.name)
    }

    pub fn template_files() -> impl Iterator<Item = DirEntry> {
        static INCLUDE: OnceLock<GlobSet> = OnceLock::new();

        let includes = INCLUDE.get_or_init(|| globset(&[
            "**/Cargo.toml",
            "**/build.rs",
            "**/src/*.rs",
        ]));

        WalkDir::new(Self::template_dir())
            .into_iter()
            .filter_entry(|e| visible(e))
            .map(|e| e.expect("entry is okay"))
            .filter(|e| includes.is_match(e.path()))
    }

    pub fn new(zball: PathBuf) -> Self {
        let tarball = zball.with_extension("").with_extension("tar");
        assert!(tarball.exists(), "tarball {tarball:?} missing");

        let name = tarball.file_stem().expect("stem").to_str().expect("string").to_owned();
        Self { name, tarball, zball }
    }

    pub fn open_tarball(&self) -> io::Result<tar::Archive<impl io::Read + io::Seek>> {
        File::open(&self.tarball)
            .map(BufReader::new)
            .map(tar::Archive::new)
    }

    pub fn copy_template_to_target(&self) -> io::Result<()> {
        println!("+ copying template {}", self.name);
        let target_dir = self.target_dir();
        std::fs::create_dir_all(&target_dir)?;

        for entry in Self::template_files() {
            let source = entry.path();
            let suffix = source.strip_prefix(Self::template_dir()).unwrap();
            let destination = target_dir.join(suffix);

            std::fs::create_dir_all(destination.parent().expect("join => parent"))?;
            std::fs::copy(source, destination)?;
        }

        println!("+ {:?} -> {:?}", self.zball.file_name().unwrap(), target_dir);
        std::fs::copy(&self.zball, target_dir.join("pack.tar.zst"))?;
        Ok(())
    }

    pub fn render_cargo_toml(&self) -> io::Result<()> {
        let toml_path = self.target_dir().join("Cargo.toml");
        let mut cargo_toml = std::fs::read_to_string(&toml_path)?
            .parse::<Document>()
            .expect("Cargo.toml");

        // Make the crate publishable.
        cargo_toml["package"]["publish"] = value(true);
        cargo_toml["package"]["name"] = value(format!("jellybean-{}", self.name));

        let languages = self.open_tarball()?
            .entries_with_seek()?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.path().ok().and_then(|p| p.iter().next().map(|v| v.to_os_string())))
            .filter_map(|name| name.into_string().ok())
            .map(|name| name.replace('-', "_"))
            .collect::<HashSet<_>>();

        cargo_toml["features"].as_table_mut().unwrap().clear();
        for language in &languages {
            cargo_toml["features"][&language] = value(Array::new());
        }

        cargo_toml["features"]["default"] = value(Array::from_iter(languages.iter()));
        std::fs::write(&toml_path, cargo_toml.to_string())
    }

    pub fn expand(&self) -> io::Result<()> {
        self.copy_template_to_target()?;
        self.render_cargo_toml()
    }

    pub fn outdated(&self) -> io::Result<bool> {
        if !self.target_dir().exists() {
            return Ok(true);
        }

        let source_mod = self.zball.metadata()?.modified()?;
        let target_mod = self.target_dir().join("Cargo.toml").metadata()?.modified()?;
        let template_mod = Self::template_files()
            .map(|entry| entry.metadata().expect("entry metadata"))
            .map(|metadata| metadata.modified().expect("modified date"))
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        Ok(source_mod >= target_mod || template_mod >= target_mod)
    }
}

pub fn main(args: &[&str]) -> io::Result<()> {
    if !PackBuilder::packs_container().exists() {
        err_exit("no packs to build - try running `package` first");
    }

    if !PackExpander::template_dir().exists() {
        err_exit("pack template is missing");
    }

    let mut threads = vec![];
    for entry in PackBuilder::packs_container().read_dir()? {
        let path = entry?.path();
        if !path.extension().map_or(false, |ext| ext == "zst") {
            continue;
        }

        let force = flag(args, "f");
        threads.push(std::thread::spawn(move || {
            let expander = PackExpander::new(path);
            if force || expander.outdated()? {
                println!(":: expanding {}", expander.name);
                expander.expand()
            } else {
                println!(":: {} is up to date", expander.name);
                Ok(())
            }
        }));
    }

    for result in threads {
        result.join().expect("thread panicked")?;
    }

    Ok(())
}
