use std::io::Write;
use std::{io, fmt};
use std::path::PathBuf;

use yansi::{Style, Color::*, Attribute::*, Quirk::*};
use jellybean::{Language, Highlight, Theme};

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
    ("label", Magenta.foreground().dim()),
    ("none", Clear.style()),
    ("operator", Magenta.bold()),
    ("property", Cyan.foreground()),
    ("punctuation", Bold.style()),
    ("punctuation.bracket", Primary.bold()),
    ("punctuation.delimiter", Primary.bold()),
    ("punctuation.special", Magenta.bold()),
    ("string", Green.bright()),
    ("string.special", Green.bright()),
    ("tag", BrightRed.foreground()),
    ("text", Primary.foreground()),
    ("text.reference", Blue.foreground()),
    ("text.strike", Strike.style()),
    ("text.title", Magenta.foreground()),
    ("text.uri", Green.underline()),
    ("type", Blue.foreground()),
    ("type.builtin", Yellow.foreground()),
    ("variable", Cyan.foreground()),
    ("variable.builtin", Yellow.foreground()),
    ("variable.parameter", Red.foreground()),
]);

fn parse_cli_args() -> PathBuf {
    let mut args = std::env::args_os();
    let binary = args.next().expect("binary name");
    let path = args.next()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let binary = binary.to_string_lossy();
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

fn write_styled_event(
    out: &mut dyn fmt::Write,
    stack: &mut Vec<Style>,
    highlight: Highlight<'_>
) -> fmt::Result {
    static DEFUALT: Style = Style::new();

    match highlight {
        Highlight::Start { group, .. } => {
            let style = THEME.find(group).unwrap_or(&DEFUALT);
            stack.push(*style);
            style.fmt_prefix(out)
        },
        Highlight::Source { text, .. } => out.write_str(text),
        Highlight::End => {
            // Restore the previous styling.
            stack.pop();
            Clear.style().fmt_suffix(out)?;
            stack.iter().try_for_each(|s| s.fmt_prefix(out))
        }
    }
}

fn main() -> io::Result<()> {
    // Get the input path from CLI. Read input file into a string.
    let input = parse_cli_args();
    let source = std::fs::read_to_string(&input)?;

    // Use the input's extension, if any, to find the associated language.
    let ext = input.extension().map(|ext| ext.to_string_lossy());
    let language = ext.as_ref()
        .and_then(|ext| Language::find(ext))
        .or_else(|| input.file_name().and_then(|f| f.to_str()).and_then(Language::find));

    // Print the source with terminal colors if we have a language highlighter.
    let mut stack = vec![];
    let output = if let Some(language) = language {
        let mut output = String::with_capacity(source.len());
        language.highlighter()
            .highlight(&source)
            .try_for_each(|event| write_styled_event(&mut output, &mut stack, event.unwrap()))
            .expect("foo bar");

        output
    } else {
        let ext = ext.unwrap_or("[empty]".into());
        eprintln!("warning: emitting plaintext ({:?} not recognized)", ext);
        source
    };

    std::io::stdout().write_all(output.as_bytes())
}
