//! A grammar for `go.sum` and `go.work.sum`, which share a format.
//! https://go.dev/ref/mod#go-sum-files

use super::common::{field, line_end, parse_entries, ws, ws1};
use winnow::Result;
use winnow::combinator::alt;
use winnow::prelude::*;

/// `<module> <version>[/go.mod] <checksum>`
#[derive(Debug, PartialEq)]
pub struct Entry<'i> {
    pub module: &'i str,
    /// Ends with `/go.mod` when it's the checksum of that version's
    /// `go.mod` file only.
    pub version: &'i str,
    pub checksum: &'i str,
}

pub fn parse(content: &str) -> Vec<Entry<'_>> {
    parse_entries(content, line).into_iter().flatten().collect()
}

/// An entry, or a blank line. Go doesn't write comments, but they're
/// allowed like in its other files.
fn line<'i>(input: &mut &'i str) -> Result<Option<Entry<'i>>> {
    alt((
        line_end.map(|()| None),
        (ws, field, ws1, field, ws1, field, line_end).map(
            |(_, module, _, version, _, checksum, _)| {
                Some(Entry {
                    module,
                    version,
                    checksum,
                })
            },
        ),
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries() {
        let content = "// Comment\n\n  example.com/a v1.0.0 h1:a//b= // Comment\nexample.com/a v1.0.0/go.mod h1:c=\nexample.com/b v1.0.0\n";

        assert_eq!(
            parse(content),
            [
                Entry {
                    module: "example.com/a",
                    version: "v1.0.0",
                    checksum: "h1:a//b=",
                },
                Entry {
                    module: "example.com/a",
                    version: "v1.0.0/go.mod",
                    checksum: "h1:c=",
                },
            ]
        );
    }
}
