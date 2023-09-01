#[macro_use]
#[path = "../../lib/shared/util.rs"]
mod util;

#[path = "../../lib/shared/language.rs"]
mod language;

mod clean;
mod fetch;
mod sync_features;

pub const USAGE: &str = r"
usage:
    cargo xtask [task args..]

example:
    cargo xtask vm --release

tasks:
                          default task: sync
    sync [-u]             run fetch-languages and sync-features (-u updates)
    fetch-languages [-u]  fetch all language sources (-u to update existing)
    sync-features         synchronize crate features with available languages
    clean                 remove all language sources and artifacts
";

/// Print `msg` and exit.
pub fn err_exit(msg: impl std::fmt::Display) -> ! {
    eprint!("error: {msg}\n{}", crate::USAGE);
    std::process::exit(1)
}

fn main() {
    macro_rules! run {
        ($task:ident, $args:expr) => {
            if let Err(e) = $task::main($args) {
                let name = stringify!($task);
                $crate::err_exit(format!("`{name}` task failed: {e}"));
            }
        }
    }

    let args = std::env::args().collect::<Vec<_>>();
    let args = args.iter().skip(1).map(|s| s.as_str()).collect::<Vec<_>>();
    match args.get(0) {
        Some(&"sync") | None => { run!(fetch, &args); run!(sync_features, &args) },
        Some(&"fetch-languages") => run!(fetch, &args),
        Some(&"sync-features") => run!(sync_features, &args),
        Some(&"clean") => run!(clean, &args),
        Some(cmd) => err_exit(format!("unknown task `{cmd}`")),
    }
}
