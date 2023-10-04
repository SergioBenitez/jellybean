use std::path::PathBuf;

use yansi::{Style, Color::*, Paint};
use jellybean::{Language, Highlight, Theme, COMMON_CAPTURES};

// This is just an arbitrary theme.
pub static THEME: Theme<Style> = Theme::new(&[
    ("attribute", Blue.foreground()),
    ("comment", BrightBlack.foreground()),
    ("constant", Red.bright()),
    ("constructor", Blue.foreground()),
    ("escape", BrightRed.foreground()),
    ("function", Blue.bright()),
    ("function.builtin", Magenta.foreground()),
    ("function.macro", Magenta.foreground()),
    ("keyword", Red.foreground()),
    ("label", Green.dim()),
    ("operator", Magenta.bold()),
    ("property", Cyan.foreground()),
    ("punctuation", Primary.bold()),
    ("punctuation.bracket", Primary.foreground()),
    ("punctuation.delimiter", Primary.bold()),
    ("string", Green.bright()),
    ("string.special", Green.bright()),
    ("tag", BrightRed.foreground()),
    ("type", Blue.foreground()),
    ("type.builtin", Yellow.foreground()),
    ("variable", Cyan.foreground()),
    ("variable.builtin", Yellow.foreground()),
    ("variable.parameter", Red.foreground()),
]);

fn parse_cli_args() -> PathBuf {
    let mut args = std::env::args_os();
    let binary = args.next().expect("binary name");
    let binary = binary.to_string_lossy();
    let path = args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            eprintln!("error: required <path> argument is missing\n");
            eprintln!("usage: {binary} <path>");
            eprintln!("example: {binary} src/main.rs");
            std::process::exit(1);
        });

    if !path.exists() {
        eprintln!("error: {:?} does not exist", path);
        std::process::exit(1);
    }

    path
}

fn print_styled_event(stack: &mut Vec<Style>, highlight: Highlight<'_>) {
    match highlight {
        Highlight::Start { group, .. } => {
            let style = THEME.find(group).copied().unwrap_or(Primary.foreground());
            stack.push(style);
            print!("{}", style.prefix());
        }
        Highlight::Source { text, .. } => print!("{}", text),
        Highlight::End => {
            // Restore the previous styling if there was some.
            stack.pop();
            if let Some(style) = stack.last() {
                print!("{}", style.prefix());
            } else {
                print!("{}", "".clear());
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    // Get the input path from CLI. Read input file into a string.
    let input = parse_cli_args();
    let source = std::fs::read_to_string(&input)?;

    // Use the input's extension, if any, to find the associated language.
    let ext = input.extension().map(|ext| ext.to_string_lossy());
    let language = ext.as_ref().and_then(|ext| Language::find(ext));

    // Print the source with terminal colors if we have a language highlighter.
    let mut stack = vec![];
    if let Some(language) = language {
        language.highlighter(COMMON_CAPTURES)
            .highlight(&source)
            .for_each(|event| print_styled_event(&mut stack, event.unwrap()))
    } else {
        let ext = ext.unwrap_or("[empty]".into());
        eprintln!("warning: emitting plaintext ({:?} not recognized)", ext);
        println!("{}", source);
    }

    Ok(())
}
