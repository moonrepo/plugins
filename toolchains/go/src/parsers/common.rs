//! Helpers shared by the `go.sum` and `go.work` grammars. These files are
//! line based, with `//` comments that only start at the beginning of a
//! token, since checksums and paths may contain `//` themselves.

use winnow::Result;
use winnow::ascii::{line_ending, space0, space1, till_line_ending};
use winnow::combinator::{alt, delimited, eof, not, opt, preceded, repeat};
use winnow::prelude::*;
use winnow::token::{any, none_of, take_till, take_while};

/// Applies `entry` until the end of the input. A line that it can't parse
/// is skipped, so one malformed line never fails the whole file.
pub fn parse_entries<'i, O>(
    content: &'i str,
    mut entry: impl Parser<&'i str, O, winnow::error::ContextError>,
) -> Vec<O> {
    let mut input = content.trim_start_matches('\u{feff}');
    let mut entries = vec![];

    while !input.is_empty() {
        let before = input;

        match entry.parse_next(&mut input) {
            Ok(output) if input.len() < before.len() => entries.push(output),
            // Skip to the next line, which always makes progress
            _ => {
                input = before;

                let _: Result<_> = (take_till(0.., '\n'), opt('\n')).parse_next(&mut input);
            }
        }
    }

    entries
}

/// Spaces and tabs.
pub fn ws(input: &mut &str) -> Result<()> {
    space0.void().parse_next(input)
}

/// At least one space or tab.
pub fn ws1(input: &mut &str) -> Result<()> {
    space1.void().parse_next(input)
}

pub fn comment(input: &mut &str) -> Result<()> {
    ("//", till_line_ending).void().parse_next(input)
}

/// The end of a line, with an optional trailing comment.
pub fn line_end(input: &mut &str) -> Result<()> {
    (ws, opt(comment), alt((line_ending.void(), eof.void())))
        .void()
        .parse_next(input)
}

/// A line with only whitespace or a comment. Unlike [`line_end`], it never
/// matches the end of the input, so it's safe to repeat.
pub fn blank_line(input: &mut &str) -> Result<()> {
    (ws, opt(comment), line_ending).void().parse_next(input)
}

/// A run of non-whitespace characters, like a module path, version or hash.
pub fn field<'i>(input: &mut &'i str) -> Result<&'i str> {
    preceded(not("//"), take_while(1.., |c: char| !c.is_whitespace())).parse_next(input)
}

/// A bare word, or a quoted (`"..."`) or raw (`` `...` ``) string,
/// as used for `go.work` arguments.
pub fn token(input: &mut &str) -> Result<String> {
    alt((
        delimited(
            '"',
            repeat(
                0..,
                alt((preceded('\\', any), none_of(['"', '\\', '\n', '\r']))),
            )
            .fold(String::new, |mut value, c| {
                value.push(c);
                value
            }),
            '"',
        ),
        delimited('`', take_till(0.., ['`', '\n']), '`').map(String::from),
        preceded(
            not("//"),
            take_while(1.., |c: char| {
                !c.is_whitespace() && !matches!(c, '(' | ')' | '"' | '`')
            }),
        )
        .map(String::from),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tokens() {
        let mut input = r#""./a b" `./c` ./d"#;
        let tokens: Vec<String> = repeat(0.., preceded(ws, token))
            .parse_next(&mut input)
            .unwrap();

        assert_eq!(tokens, ["./a b", "./c", "./d"]);
    }

    #[test]
    fn keeps_double_slashes_within_fields() {
        let mut input = "h1:X2//Uz= // comment";

        assert_eq!(field.parse_next(&mut input).unwrap(), "h1:X2//Uz=");
        assert!(line_end.parse_next(&mut input).is_ok());
    }

    #[test]
    fn skips_lines_it_cant_parse() {
        let word = take_while(1.., char::is_alphabetic);
        let entries = parse_entries("a\n1 2\nb\r\nc", (word, line_end).map(|(w, _)| w));

        assert_eq!(entries, ["a", "b", "c"]);
    }
}
