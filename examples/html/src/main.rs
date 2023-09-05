use std::io;
use std::fmt::Write;
use std::path::PathBuf;
use std::fs::File;

use jellybean::{Language, Highlight, COMMON_CAPTURES};

const HTML_PREFIX: &str = concat!(r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <title>Jellybean Syntax Highlighted HTML</title>
        <meta charset="UTF-8">
        <style>"#,
        include_str!("theme.css"),
        r#"</style>
    </head>
    <body>
"#);

const HTML_SUFFIX: &str = r#"
    </body>
</html>
"#;

/// Parse the CLI arguments. Expects and returns an input and output path.
fn parse_cli_args() -> io::Result<(PathBuf, Box<dyn std::io::Write>)> {
    let mut args = std::env::args_os();
    let binary = args.next().expect("binary name");
    let binary = binary.to_string_lossy();
    let (input, output) = match (args.next(), args.next()) {
        (Some(input), Some(output)) => (PathBuf::from(input), Box::new(File::create(output)?) as _),
        (Some(input), None) => (PathBuf::from(input), Box::new(io::stdout()) as _),
        _ => {
            eprintln!("error: required <input> argument missing\n");
            eprintln!("usage: {binary} <input> [output]");
            eprintln!("example: {binary} src/main.rs");
            eprintln!("example: {binary} src/main.rs /tmp/output.html");
            std::process::exit(1);
        }
    };

    if !input.exists() {
        eprintln!("error: {:?} does not exist", input);
        std::process::exit(1);
    }

    Ok((input, output))
}

fn html_prefix(source: &str) -> String {
    let mut html = String::new();
    html.push_str(HTML_PREFIX);
    html.push_str("<div class=\"code container\" style=\"display: flex;\">");
    html.push_str("<pre class=\"line-nums\">");

    let lines = memchr::memrchr_iter(b'\n', source.as_bytes()).count();
    for i in 1..=lines {
        if i < lines { let _ = write!(&mut html, "{}\n", i); }
        else { let _ = write!(&mut html, "{}", i); }
    }

    html.push_str("</pre>");
    html.push_str("<pre class=\"code\">");
    html
}

fn html_lines_unstyled(html: &mut String, source: &str) {
    let _ = write!(html, "{}", v_htmlescape::escape(source));
}

fn html_line_styled(html: &mut String, highlight: Highlight<'_>) {
    match highlight {
        Highlight::Start { group, .. } => {
            html.push_str("<span class='");

            let mut highlights = group.split('.').peekable();
            while let Some(hl) = highlights.next() {
                html.push_str(hl);
                if highlights.peek().is_some() {
                    html.push_str(" ");
                }
            }

            html.push_str("'>");
        },
        Highlight::Source { text, .. } => {
            let _ = write!(html, "{}", v_htmlescape::escape(text));
        },
        Highlight::End => {
            html.push_str("</span>");
        }
    }
}

fn html_finalize(html: &mut String) {
    html.push_str(HTML_SUFFIX);
}

fn main() -> std::io::Result<()> {
    // Get the input and output path from CLI. Read input file into a string.
    let (input, mut output) = parse_cli_args()?;
    let source = std::fs::read_to_string(&input)?;

    // Use the input's extension, if any, to find the associated language.
    let ext = input.extension().map(|ext| ext.to_string_lossy());
    let language = ext.as_ref().and_then(|ext| Language::find(ext));

    // Generate the HTML, using the language's highlighter.
    let mut html = html_prefix(&source);
    if let Some(language) = language {
        for event in language.highlighter(COMMON_CAPTURES).highlight(&source) {
            html_line_styled(&mut html, event.unwrap());
        }
    } else {
        let ext = ext.unwrap_or("[empty]".into());
        eprintln!("warning: emitting unstyled HTML ({:?} not recognized)", ext);
        html_lines_unstyled(&mut html, &source);
    }

    html_finalize(&mut html);
    output.write_all(html.as_bytes())
}
