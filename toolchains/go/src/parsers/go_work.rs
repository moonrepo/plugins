//! A grammar for `go.work`: directives with arguments, and blocks like
//! `use ( ... )`, with bare, quoted (`"..."`) or raw (`` `...` ``) arguments.
//! https://go.dev/ref/mod#go-work-file

use super::common::{blank_line, line_end, parse_entries, token, ws};
use winnow::Result;
use winnow::combinator::{alt, delimited, not, preceded, repeat};
use winnow::prelude::*;

/// A directive like `use ./a`. A line within a block, like `use ( ... )`,
/// becomes a directive with the block's name.
#[derive(Debug, PartialEq)]
pub struct Directive {
    pub name: String,
    pub args: Vec<String>,
}

pub fn parse(content: &str) -> Vec<Directive> {
    parse_entries(content, entry)
        .into_iter()
        .flatten()
        .collect()
}

fn entry(input: &mut &str) -> Result<Vec<Directive>> {
    alt((
        line_end.map(|()| vec![]),
        block,
        directive.map(|directive| vec![directive]),
    ))
    .parse_next(input)
}

/// `name arg...`
fn directive(input: &mut &str) -> Result<Directive> {
    (ws, token, args, line_end)
        .map(|(_, name, args, _)| Directive { name, args })
        .parse_next(input)
}

/// `name (`, then a directive per line, and a closing `)`.
fn block(input: &mut &str) -> Result<Vec<Directive>> {
    let name = delimited(ws, token, (ws, '(', line_end)).parse_next(input)?;

    let lines: Vec<Option<Vec<String>>> = repeat(
        0..,
        alt((
            blank_line.map(|()| None),
            delimited((ws, not(')')), (token, args), line_end).map(|(first, mut rest)| {
                rest.insert(0, first);
                Some(rest)
            }),
        )),
    )
    .parse_next(input)?;

    (ws, ')', line_end).parse_next(input)?;

    Ok(lines
        .into_iter()
        .flatten()
        .map(|args| Directive {
            name: name.clone(),
            args,
        })
        .collect())
}

fn args(input: &mut &str) -> Result<Vec<String>> {
    repeat(0.., preceded(ws, token)).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directive(name: &str, args: &[&str]) -> Directive {
        Directive {
            name: name.into(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
        }
    }

    #[test]
    fn parses_directives_and_blocks() {
        let content = "go 1.24 // Comment\ntoolchain go1.26.0\n\nreplace (\n\texample.com/a v1.0.0 => ../a\n\n\t// Comment\n\texample.com/b => example.com/c v1.2.0\n)\n";

        assert_eq!(
            parse(content),
            [
                directive("go", &["1.24"]),
                directive("toolchain", &["go1.26.0"]),
                directive("replace", &["example.com/a", "v1.0.0", "=>", "../a"]),
                directive(
                    "replace",
                    &["example.com/b", "=>", "example.com/c", "v1.2.0"]
                ),
            ]
        );
    }

    #[test]
    fn skips_malformed_lines() {
        let content = "use ( ./inline )\nuse \"./unterminated\nuse ./ok\n)\n";

        assert_eq!(parse(content), [directive("use", &["./ok"])]);
    }

    #[test]
    fn parses_a_block_at_the_end_without_a_trailing_newline() {
        assert_eq!(parse("use (\r\n\t./a\r\n)"), [directive("use", &["./a"])]);
    }
}
