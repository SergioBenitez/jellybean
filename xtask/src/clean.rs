use std::io;

use crate::language::Language;

pub fn main(_: &[&str]) -> io::Result<()> {
    if Language::checkout_dir().exists() {
        std::fs::remove_dir_all(Language::checkout_dir())?;
    }

    crate::sync_features::sync([])
}
