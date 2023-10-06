use std::fs::File;
use std::io::{self, BufWriter, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use globset::GlobSet;
use indicatif::{ProgressBar, MultiProgress, ProgressStyle, ProgressFinish};
use walkdir::{WalkDir, DirEntry};

use crate::fetch::TsLanguage;
use crate::util::{visible, globset, diff_paths, flag};
use crate::{crate_path, vprintln};

#[derive(Default)]
pub struct PackBuilder {
    packs: Vec<PackArchive>,
    current: Option<PackArchive>,
}

pub struct PackArchive {
    sequence: usize,
    /// The path to the tarball. For compressed, use `.zball_path()`.
    path: PathBuf,
    builder: tar::Builder<BufWriter<File>>,
    size: u64,
}

impl PackBuilder {
    const INCLUDE: &'static [&'static str] = &[
        "**/*.h",
        "**/package.json",
        "**/src/parser.c",
        "**/src/scanner.c",
        "**/src/scanner.cc",
        "**/queries/locals.scm",
        "**/queries/highlights.scm",
        "**/queries/injections.scm",
    ];

    const EXCLUDE: &'static [&'static str] = &[
        "**/bin/**",
        "**/node_modules/**",
        "**/bindings/**",
    ];

    // 350 MiB. At ~2% compression, we get ~7MiB packs.
    const MAX_ARCHIVE_SIZE: u64 = 300 << 20;

    const PROGRESS_TEMPLATE: &'static str = "{spinner} {prefix}: \
        {bar:40.cyan/blue} {percent:>3}% ({binary_bytes_per_sec} - {eta} {msg})";

    pub fn includes() -> &'static GlobSet {
        static CELL: OnceLock<GlobSet> = OnceLock::new();
        CELL.get_or_init(|| globset(Self::INCLUDE))
    }

    pub fn excludes() -> &'static GlobSet {
        static CELL: OnceLock<GlobSet> = OnceLock::new();
        CELL.get_or_init(|| globset(Self::EXCLUDE))
    }

    pub fn packs_container() -> &'static Path {
        crate_path!("artifacts", "packs")
    }

    /// Returns the seauence number for the next pack.
    fn finalize_current_pack(&mut self) -> io::Result<usize> {
        if let Some(mut current) = self.current.take() {
            let sequence = current.sequence;
            current.builder.finish()?;
            current.builder.get_mut().flush()?;
            self.packs.push(current);
            return Ok(sequence + 1);
        }

        Ok(0)
    }

    fn rotate(&mut self) -> io::Result<&mut PackArchive> {
        if let Some(current) = self.current.as_mut() {
            if current.size < Self::MAX_ARCHIVE_SIZE {
                return Ok(self.current.as_mut().unwrap());
            }
        }

        let seq = self.finalize_current_pack()?;
        let path = Self::packs_container().join(format!("pack-{seq}.tar"));
        let writer = BufWriter::new(File::create(&path)?);

        self.current = Some(PackArchive {
            sequence: seq,
            path: path,
            builder: tar::Builder::new(writer),
            size: 0,
        });

        // println!("+ {}", self.current.as_ref().unwrap().path.display());
        Ok(self.current.as_mut().unwrap())
    }

    pub fn pack_sources() -> impl Iterator<Item = (TsLanguage, impl Iterator<Item = DirEntry>)> {
        TsLanguage::iter().map(|language| {
            let files = WalkDir::new(language.checkout_path())
                .into_iter()
                .filter_entry(|e| visible(e) && !Self::excludes().is_match(e.path()))
                .map(|e| e.expect("entry is okay"))
                .filter(|e| Self::includes().is_match(e.path()));

            (language, files)
        })
    }

    pub fn tar_packs(&mut self) -> io::Result<()> {
        std::fs::create_dir_all(Self::packs_container())?;

        for (_language, entries) in Self::pack_sources() {
            let archive = self.rotate()?;
            for entry in entries {
                archive.add(entry.path())?;
            }
        }

        self.finalize_current_pack()?;
        Ok(())
    }

    pub fn compress_packs(&mut self) -> io::Result<()> {
        let mut threads = vec![];
        let multiprogress = MultiProgress::new();
        let style = ProgressStyle::with_template(Self::PROGRESS_TEMPLATE).unwrap();
        println!(":: compressing {} pack archives", self.packs.len());

        for pack in self.packs.iter() {
            let style = style.clone();
            let multiprogress = multiprogress.clone();
            let path = pack.path.clone();
            let zball = pack.zball_path();
            threads.push(std::thread::spawn(move || {
                let tar = BufReader::new(File::open(&path)?);
                let output = BufWriter::new(File::create(&zball)?);

                let bar = ProgressBar::new(tar.get_ref().metadata()?.len())
                    .with_style(style)
                    .with_prefix(format!("{}", zball.file_name().unwrap().to_string_lossy()))
                    .with_message("left")
                    .with_finish(ProgressFinish::WithMessage("✓".into()));

                multiprogress.add(bar.clone());
                zstd::stream::copy_encode(bar.wrap_read(tar), output, 19)
            }));
        }

        for result in threads {
            result.join().expect("zstd thread panicked")?;
        }

        Ok(())
    }

    pub fn packs_outdated() -> io::Result<bool> {
        if !Self::packs_container().exists() {
            return Ok(true);
        }

        let max_source_date = Self::pack_sources()
            .flat_map(|(_, entries)| entries)
            .map(|entry| entry.metadata().expect("entry metadata"))
            .map(|metadata| metadata.modified().expect("modified date"))
            .max();

        let max_pack_date = Self::packs_container()
            .read_dir()?
            .map(|entry| entry.expect("pack entry"))
            .map(|entry| entry.metadata().expect("pack metadata"))
            .map(|metadata| metadata.modified().expect("modified date"))
            .max();

        Ok(max_source_date >= max_pack_date)
    }
}

impl PackArchive {
    pub fn add(&mut self, path: &Path) -> io::Result<()> {
        let name = diff_paths(path, TsLanguage::checkout_container()).unwrap();

        vprintln!("+ {}", name.display());
        self.size += path.metadata()?.len();
        self.builder.append_path_with_name(path, name)
    }

    pub fn zball_path(&self) -> PathBuf {
        self.path.with_extension("tar.zst")
    }
}

pub fn main(args: &[&str]) -> io::Result<()> {
    if flag(args, "f") || PackBuilder::packs_outdated()? {
        let mut builder = PackBuilder::default();
        builder.tar_packs()?;
        builder.compress_packs()?;
        println!(":: done")
    } else {
        println!(":: packs are up to date")
    }

    Ok(())
}
