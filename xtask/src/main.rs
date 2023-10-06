mod util;
mod clean;
mod fetch;
mod package;
mod expand;
mod sync;

pub const USAGE: &str = r"
usage:
    cargo xtask [task args..]

examples:
    cargo xtask
    cargo xtask clean

tasks: [default: fetch + package + expand + sync]
    help                  display this help message
    fetch [-u]            fetch all language sources (-u to update existing)
    package [-u, -f]      fetch and compress into packs (-u to update, -f to force)
    expand [-f]           expand existing packs into crates (-f to force)
    sync                  synchronize jellybean lib metadata with packs
    clean                 remove all fetched sources and package artifacts
";

// sync [-u] (default)   run fetch-languages and sync-features (-u updates)
// sync-features         sync crate features with available languages

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

    let verbose = crate::util::flag(&args, "v");
    crate::util::VERBOSE.store(verbose, std::sync::atomic::Ordering::Relaxed);

    let help = crate::util::flag(&args, "h");
    let cmd = args.get(0).and_then(|v| (!v.starts_with('-')).then_some(v));
    match cmd {
        Some(&"help") | _ if help => err_exit("jellybean xtask help"),
        Some(&"fetch") => run!(fetch, &args),
        Some(&"clean") => run!(clean, &args),
        Some(&"expand") => run!(expand, &args),
        Some(&"sync") => run!(sync, &args),
        Some(&"package") => {
            run!(fetch, &args);
            run!(package, &args);
        }
        None => {
            run!(fetch, &args);
            run!(package, &args);
            run!(expand, &args);
            run!(sync, &args);
        }
        Some(cmd) => err_exit(format!("unknown task `{cmd}`")),
    }
}
