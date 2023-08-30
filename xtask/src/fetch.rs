use std::io;

use crate::language::Language;

pub fn main(args: &[&str]) -> io::Result<()> {
    let update_existing = args.get(1) == Some(&"-u");
    println!(":: fetching languages (updating? {update_existing})");

    let csv_text = Language::read_source()?;
    for lang in Language::parse_source_text(&csv_text) {
        let lang_dir = lang.local_checkout_dir();
        if lang_dir.exists() {
            if args.get(1) == Some(&"-u") {
                println!("= {} ({}:{})", lang.name, lang.git_url, lang.branch.unwrap_or("default"));
                cmd!(lang_dir => "git", "pull")?;
            }
        } else {
            println!("+ {} ({}:{})", lang.name, lang.git_url, lang.branch.unwrap_or("default"));
            cmd! {
                "git", "clone",
                #lang.branch.map(|b| vec!["-b", b]).unwrap_or_default(),
                "--depth=1",
                lang.git_url,
                lang_dir
            }?;
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
