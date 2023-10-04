use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{Ordering, AtomicU8};

use globset::{GlobSet, GlobSetBuilder, Glob};

#[macro_export]
macro_rules! cmd {
    ($bin:expr) => (cmd!($bin,));
    ($cwd:expr => $bin:expr) => (cmd!($cwd => $bin,));
    ($cwd:expr => $bin:expr, $($token:tt)*) => {{
        let mut cmd = std::process::Command::new($bin);
        cmd.current_dir($cwd);
        cmd!(@arg[cmd] $($token)*);
        cmd.spawn().and_then(|mut p| match p.wait() {
            Ok(e) if e.success() => Ok(()),
            Ok(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            Err(e) => Err(e)
        })
    }};
    ($bin:expr, $($token:tt)*) => {{
        let mut cmd = std::process::Command::new($bin);
        cmd!(@arg[cmd] $($token)*);
        cmd.spawn().and_then(|mut p| match p.wait() {
            Ok(e) if e.success() => Ok(()),
            Ok(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            Err(e) => Err(e)
        })
    }};
    (@arg[$cmd:expr] #$args:expr) => ($cmd.args($args));
    (@arg[$cmd:expr] #$args:expr, $($token:tt)*) => {
        cmd!(@arg[$cmd] #$args);
        cmd!(@arg[$cmd] $($token)*);
    };
    (@arg[$cmd:expr] $arg:expr) => ($cmd.arg($arg));
    (@arg[$cmd:expr] $arg:expr, $($token:tt)*) => {
        cmd!(@arg[$cmd] $arg);
        cmd!(@arg[$cmd] $($token)*);
    };
    (@arg[$cmd:expr] $(,)*) => ();
}

#[macro_export] #[cfg(windows)] macro_rules! slash { () => { r#"\"# } }
#[macro_export] #[cfg(not(windows))] macro_rules! slash { () => {   "/"  } }

#[macro_export]
macro_rules! path_str {
    ($root:expr $(, $path:literal)*) => {
        concat!($root, $($crate::slash!(), $path),*)
   }
}

#[macro_export]
macro_rules! path {
    ($root:expr $(, $path:literal)*) => {
        std::path::Path::new($crate::path_str!($root $(, $path)*))
   }
}

#[macro_export]
macro_rules! crate_path {
    ($($path:literal),*) => {
        $crate::path!(std::env!("CARGO_MANIFEST_DIR"), $($path),*)
   }
}

#[macro_export]
macro_rules! vprintln {
    ($verbose_flag:expr, $($token:tt)*) => {
        if $verbose_flag {
            println!($($token)*);
        }
   }
}

pub fn visible(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| !s.starts_with("."))
        .unwrap_or(true)
}

pub fn globset(patterns: &[&str]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    patterns.iter().for_each(|path| { builder.add(Glob::new(path).unwrap()); });
    builder.build().unwrap()
}

/// A helper function to determine the relative path to `path` from `base`.
///
/// Returns `None` if there is no relative path from `base` to `path`, that is,
/// `base` and `path` do not share a common ancestor. `path` and `base` must be
/// either both absolute or both relative; returns `None` if one is relative and
/// the other absolute.
///
/// ```
/// use std::path::Path;
/// use figment::util::diff_paths;
///
/// // Paths must be both relative or both absolute.
/// assert_eq!(diff_paths("/a/b/c", "b/c"), None);
/// assert_eq!(diff_paths("a/b/c", "/b/c"), None);
///
/// // The root/relative root is always a common ancestor.
/// assert_eq!(diff_paths("/a/b/c", "/b/c"), Some("../../a/b/c".into()));
/// assert_eq!(diff_paths("c/a", "b/c/a"), Some("../../../c/a".into()));
///
/// let bar = "/foo/bar";
/// let baz = "/foo/bar/baz";
/// let quux = "/foo/bar/quux";
///
/// assert_eq!(diff_paths(bar, baz), Some("../".into()));
/// assert_eq!(diff_paths(baz, bar), Some("baz".into()));
/// assert_eq!(diff_paths(quux, baz), Some("../quux".into()));
/// assert_eq!(diff_paths(baz, quux), Some("../baz".into()));
/// assert_eq!(diff_paths(bar, quux), Some("../".into()));
/// assert_eq!(diff_paths(baz, bar), Some("baz".into()));
/// ```
// Copyright 2012-2015 The Rust Project Developers.
// Copyright 2017 The Rust Project Developers.
// Adapted from `pathdiff`, which itself adapted from rustc's path_relative_from.
pub fn diff_paths<P, B>(path: P, base: B) -> Option<PathBuf>
     where P: AsRef<Path>, B: AsRef<Path>
{
    let (path, base) = (path.as_ref(), base.as_ref());
    if path.has_root() != base.has_root() {
        return None;
    }

    let mut ita = path.components();
    let mut itb = base.components();
    let mut comps: Vec<Component> = vec![];
    loop {
        match (ita.next(), itb.next()) {
            (None, None) => break,
            (Some(a), None) => {
                comps.push(a);
                comps.extend(ita.by_ref());
                break;
            }
            (None, _) => comps.push(Component::ParentDir),
            (Some(a), Some(b)) if comps.is_empty() && a == b => (),
            (Some(a), Some(b)) if b == Component::CurDir => comps.push(a),
            (Some(_), Some(b)) if b == Component::ParentDir => return None,
            (Some(a), Some(_)) => {
                comps.push(Component::ParentDir);
                for _ in itb {
                    comps.push(Component::ParentDir);
                }
                comps.push(a);
                comps.extend(ita.by_ref());
                break;
            }
        }
    }

    Some(comps.iter().map(|c| c.as_os_str()).collect())
}

#[derive(Clone)]
pub struct Semaphore(Arc<AtomicU8>, u8);

#[derive(Clone)]
pub struct Token(Arc<AtomicU8>);

impl Semaphore {
    pub fn new(n: u8) -> Self {
        Semaphore(Arc::new(AtomicU8::new(0)), n)
    }

    pub fn take(&self) -> Token {
        while self.0.fetch_add(1, Ordering::AcqRel) >= self.1 {
            self.0.fetch_sub(1, Ordering::AcqRel);
            std::thread::yield_now();
        }

        Token(self.0.clone())
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Returns `true` iff `args` has the letter argument `arg`.
///
/// Examples:
///     arg(&["-f", "-u", "-ab"], "f") == true
///     arg(&["-f", "-u", "-ab"], "a") == true
///     arg(&["-f", "-u", "-ab"], "b") == true
///     arg(&["-f", "-u", "-ab"], "c") == false
pub fn flag(args: &[&str], arg: &str) -> bool {
    args.iter()
        .filter_map(|v| v.strip_prefix("-"))
        .filter(|v| v.starts_with(|c: char| c.is_ascii_alphabetic()))
        .any(|v| v.contains(arg))
}
