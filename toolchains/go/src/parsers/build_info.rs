//! A grammar for the output of `go version -m <binary>`, which lists the
//! module a Go binary was built from as `mod <path> <version> [<checksum>]`.

use super::common::{field, line_end, parse_entries, ws, ws1};
use winnow::Result;
use winnow::combinator::opt;
use winnow::prelude::*;

/// The version of the module the binary was built from.
pub fn parse_module_version(output: &str) -> Option<&str> {
    parse_entries(output, mod_line).into_iter().next()
}

/// Other lines, like `path`, `dep` and `build`, are skipped.
fn mod_line<'i>(input: &mut &'i str) -> Result<&'i str> {
    (
        ws,
        "mod",
        ws1,
        field,
        ws1,
        field,
        opt((ws1, field)),
        line_end,
    )
        .map(|(_, _, _, _, _, version, _, _)| version)
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_module_version() {
        let output = "/home/user/go/bin/gopls: go1.25.9\n\tpath\tgolang.org/x/tools/gopls\n\tmod\tgolang.org/x/tools/gopls\tv0.21.1\th1:1/o9z5Brdero4jFm9Jr46Uwj8GU9lQdoSXHMlwRHb/w=\n\tdep\tgithub.com/BurntSushi/toml\tv1.5.0\th1:W5quZX=\n";

        assert_eq!(parse_module_version(output), Some("v0.21.1"));
    }

    #[test]
    fn parses_a_local_build() {
        let output =
            "./hello: go1.26.2\n\tpath\texample.com/hello\n\tmod\texample.com/hello\t(devel)\t\n";

        assert_eq!(parse_module_version(output), Some("(devel)"));
    }

    #[test]
    fn returns_nothing_without_a_module() {
        assert_eq!(
            parse_module_version("./ls: go1.26.2\n\tpath\tcommand-line-arguments\n"),
            None
        );
    }
}
