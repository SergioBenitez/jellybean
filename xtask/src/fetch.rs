use std::io;

use walkdir::{WalkDir, DirEntry};

use crate::language::Language;

fn visible(entry: &DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| !s.starts_with("."))
        .unwrap_or(true)
}

pub fn main(args: &[&str]) -> io::Result<()> {
    let update_existing = args.get(1) == Some(&"-u");
    println!(":: fetching languages (updating? {update_existing})");

    let csv_text = Language::read_source()?;
    for lang in Language::parse_source_text(&csv_text) {
        let lang_dir = lang.local_checkout_dir();
        if lang_dir.exists() {
            if args.get(1) == Some(&"-u") {
                println!("= {} ({}:{})", lang.name, lang.git_url, lang.branch.unwrap_or("default"));
                cmd!(&lang_dir => "git", "pull")?;
            }
        } else {
            println!("+ {} ({}:{})", lang.name, lang.git_url, lang.branch.unwrap_or("default"));
            cmd! {
                "git", "clone",
                #lang.branch.map(|b| vec!["-b", b]).unwrap_or_default(),
                "--depth=1",
                lang.git_url,
                &lang_dir
            }?;
        }

        // Remove any `Cargo.toml` so `cargo publish` doesn't ignore the dir.
        let walker = WalkDir::new(&lang_dir).max_depth(3).into_iter();
        for entry in walker.filter_entry(visible) {
            let entry = entry?;
            if entry.file_name() == "Cargo.toml" {
                println!("- removing {}", entry.path().display());
                std::fs::remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}

// cmd! {
//     "git", "config", "-f", ".gitmodules",
//     format!("submodule.{}.shallow", language_dir.display()), "true"
// }?;
// cmd! {
//     "git", "submodule", "add",
//     #lang.branch
//         .map(|branch| ["-b", branch])
//         .unwrap_or(["--depth", "1"]),
//     lang.git_url,
//     language_dir
// }?;

// cmd!("git", "add", ".gitmodules")?;
// cmd!("git", "submodule", "sync")?;
// cmd!("git", "submodule", "init")?;
// cmd!("git", "submodule", "update", "--remote")?;
