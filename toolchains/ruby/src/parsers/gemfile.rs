//! A winnow grammar for the subset of Ruby that Gemfiles and gemspecs are
//! written in: method calls with literal arguments, `do`/`{}` blocks, and
//! attribute assignments. Anything else, like conditionals or expressions,
//! becomes an [`Statement::Other`] that keeps the statements of its blocks,
//! so an unsupported line never fails the whole file.

use winnow::Result;
use winnow::ascii::{line_ending, till_line_ending};
use winnow::combinator::{
    alt, delimited, empty, eof, not, opt, peek, preceded, repeat, separated, terminated,
};
use winnow::prelude::*;
use winnow::token::{any, none_of, one_of, take_till, take_while};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Symbol(String),
    Array(Vec<Value>),
    Bool(bool),
    /// Any other expression, like a constant, method call or interpolated string.
    Other,
}

impl Value {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Call {
    /// Like `spec` in `spec.add_dependency`.
    pub receiver: Option<String>,
    pub name: String,
    pub args: Vec<Value>,
    /// Hash arguments, like `key: value`, `:key => value` and `"key" => value`.
    pub options: Vec<(String, Value)>,
    pub block: Option<Vec<Statement>>,
}

impl Call {
    pub fn get_option(&self, key: &str) -> Option<&Value> {
        self.options
            .iter()
            .find_map(|(name, value)| (name == key).then_some(value))
    }
}

#[derive(Debug, PartialEq)]
pub enum Statement {
    Call(Call),
    /// Like `spec.name = "billing"`.
    Assign {
        receiver: Option<String>,
        name: String,
        value: Value,
    },
    /// Anything else, with the statements of its blocks or branches.
    Other(Vec<Statement>),
}

enum Arg {
    Positional(Value),
    Option(String, Value),
}

pub fn parse(content: &str) -> Vec<Statement> {
    let mut input = content.trim_start_matches('\u{feff}');
    let mut statements = vec![];

    loop {
        statements.extend(body.parse_next(&mut input).unwrap_or_default());

        // Skip a stray `end` or bracket, which closes nothing
        if alt((keyword("end").void(), any.void()))
            .parse_next(&mut input)
            .is_err()
        {
            break;
        }
    }

    statements
}

/// Statements up to a closing `end` or `}`, which the caller consumes.
fn body(input: &mut &str) -> Result<Vec<Statement>> {
    terminated(repeat(0.., preceded(separators, statement)), separators).parse_next(input)
}

fn statement(input: &mut &str) -> Result<Statement> {
    alt((
        control.map(Statement::Other),
        assign,
        call.map(Statement::Call),
        other_statement.map(Statement::Other),
    ))
    .parse_next(input)
}

/// `if`, `unless`, `def` and other constructs that are closed by `end`.
/// Their branches are parsed as statements, where `else` and `when`
/// lines become other statements.
fn control(input: &mut &str) -> Result<Vec<Statement>> {
    let keywords = [
        "if", "unless", "case", "while", "until", "for", "begin", "def", "class", "module",
    ];

    delimited(
        (alt(keywords.map(keyword)), opt(tokens(false))),
        body,
        keyword("end"),
    )
    .parse_next(input)
}

fn assign(input: &mut &str) -> Result<Statement> {
    let (receiver, name) = method_path.parse_next(input)?;

    let value = delimited(
        (ws, '=', not(one_of(['=', '~', '>'])), ws_nl),
        value,
        statement_end,
    )
    .parse_next(input)?;

    Ok(Statement::Assign {
        receiver,
        name,
        value,
    })
}

fn call(input: &mut &str) -> Result<Call> {
    let (receiver, name) = method_path.parse_next(input)?;

    let args = alt((
        paren_args,
        // A brace after a space is a block, not a hash
        preceded((one_of([' ', '\t']), ws, not('{')), command_args),
        empty.map(|()| vec![]),
    ))
    .parse_next(input)?;

    let block = opt(preceded(ws, block)).parse_next(input)?;

    statement_end.parse_next(input)?;

    let mut call = Call {
        receiver,
        name,
        block,
        ..Default::default()
    };

    for arg in args {
        match arg {
            Arg::Positional(value) => call.args.push(value),
            Arg::Option(key, value) => call.options.push((key, value)),
        }
    }

    Ok(call)
}

fn other_statement(input: &mut &str) -> Result<Vec<Statement>> {
    preceded(
        not(alt((
            keyword("end").void(),
            one_of([')', ']', '}']).void(),
            eof.void(),
        ))),
        tokens(true),
    )
    .parse_next(input)
}

/// A method name with an optional receiver, like `gem`, `spec.add_dependency`
/// or `Gem::Specification.new`.
fn method_path(input: &mut &str) -> Result<(Option<String>, String)> {
    let segment = (ident, repeat::<_, _, (), _, _>(0.., ("::", ident))).take();
    let mut path: Vec<&str> =
        preceded(not(reserved), separated(1.., segment, '.')).parse_next(input)?;

    let name = path.pop().unwrap_or_default().to_owned();
    let receiver = (!path.is_empty()).then(|| path.join("."));

    Ok((receiver, name))
}

fn paren_args(input: &mut &str) -> Result<Vec<Arg>> {
    delimited(
        ('(', ws_nl),
        separated(0.., arg, (ws_nl, ',', ws_nl)),
        (ws_nl, opt(','), ws_nl, ')'),
    )
    .parse_next(input)
}

/// Arguments without parentheses, which may continue onto the
/// next line after a comma.
fn command_args(input: &mut &str) -> Result<Vec<Arg>> {
    separated(1.., arg, (ws, ',', ws_nl)).parse_next(input)
}

fn arg(input: &mut &str) -> Result<Arg> {
    alt((
        option.map(|(key, value)| Arg::Option(key, value)),
        value.map(Arg::Positional),
    ))
    .parse_next(input)
}

fn option(input: &mut &str) -> Result<(String, Value)> {
    let key = alt((
        // `key: value` and `"key": value`
        terminated(
            alt((ident.map(str::to_owned), string_literal)),
            (':', not(':'), ws_nl),
        ),
        // `:key => value` and `"key" => value`
        terminated(
            alt((
                preceded(':', alt((ident.map(str::to_owned), string_literal))),
                string_literal,
            )),
            (ws, "=>", ws_nl),
        ),
    ))
    .parse_next(input)?;

    Ok((key, value.parse_next(input)?))
}

/// A block, with or without parameters, like `do |spec| ... end`.
fn block(input: &mut &str) -> Result<Vec<Statement>> {
    alt((do_block, delimited(('{', ws, opt(block_params)), body, '}'))).parse_next(input)
}

fn do_block(input: &mut &str) -> Result<Vec<Statement>> {
    delimited((keyword("do"), ws, opt(block_params)), body, keyword("end")).parse_next(input)
}

fn block_params(input: &mut &str) -> Result<()> {
    ('|', take_till(0.., '|'), '|').void().parse_next(input)
}

/// The end of a statement, which may have an `if` or `unless` modifier.
fn statement_end(input: &mut &str) -> Result<()> {
    let modifier = (
        ws,
        alt(["if", "unless", "while", "until", "rescue"].map(keyword)),
        tokens(false),
    );

    (
        opt(modifier),
        ws,
        peek(alt((
            line_ending.void(),
            one_of([';', '#', '}']).void(),
            keyword("end").void(),
            eof.void(),
        ))),
    )
        .void()
        .parse_next(input)
}

fn value(input: &mut &str) -> Result<Value> {
    alt((
        terminated(literal, peek(value_end)),
        expression.value(Value::Other),
    ))
    .parse_next(input)
}

fn literal(input: &mut &str) -> Result<Value> {
    alt((
        // `.freeze` is common in gemspecs, and keeps the value
        terminated(string, opt(".freeze")).map(|value| value.map_or(Value::Other, Value::String)),
        preceded(':', alt((ident.map(|name| Some(name.to_owned())), string)))
            .map(|value| value.map_or(Value::Other, Value::Symbol)),
        delimited(
            ('[', ws_nl),
            separated(0.., value, (ws_nl, ',', ws_nl)),
            (ws_nl, opt(','), ws_nl, ']'),
        )
        .map(Value::Array),
        keyword("true").value(Value::Bool(true)),
        keyword("false").value(Value::Bool(false)),
    ))
    .parse_next(input)
}

fn value_end(input: &mut &str) -> Result<()> {
    (
        ws,
        alt((
            line_ending.void(),
            one_of([',', ')', ']', '}', ';', '#']).void(),
            stop_keyword.void(),
            eof.void(),
        )),
    )
        .void()
        .parse_next(input)
}

/// Any other expression, up to the end of its argument.
fn expression(input: &mut &str) -> Result<()> {
    (
        expression_token,
        repeat::<_, _, (), _, _>(0.., preceded(ws, expression_token)),
    )
        .void()
        .parse_next(input)
}

fn expression_token(input: &mut &str) -> Result<()> {
    alt((
        string.void(),
        bracketed,
        preceded(not(stop_keyword), ident).void(),
        // Operators and numbers, as words are identifiers
        one_of(|c: char| !(is_word_start(c) || " \t\r\n,;#()[]{}\"'".contains(c))).void(),
    ))
    .parse_next(input)
}

/// The rest of a statement that's not otherwise supported, including
/// the statements of a `do` block when `blocks` is set.
fn tokens<'i>(blocks: bool) -> impl Parser<&'i str, Vec<Statement>, winnow::error::ContextError> {
    repeat(
        1..,
        preceded(ws, move |input: &mut &'i str| {
            if blocks && let Some(body) = opt(do_block).parse_next(input)? {
                return Ok(body);
            }

            alt((
                string.void(),
                bracketed,
                preceded(not(keyword("end")), ident).void(),
                // Includes an opening bracket or quote that isn't closed
                one_of(|c: char| !(is_word_start(c) || " \t\r\n;#)]}".contains(c))).void(),
            ))
            .map(|()| vec![])
            .parse_next(input)
        }),
    )
    .fold(Vec::new, |mut statements, body| {
        statements.extend(body);
        statements
    })
}

/// A bracketed group, like a call's arguments or a hash, which may span lines.
fn bracketed(input: &mut &str) -> Result<()> {
    let contents = |input: &mut &str| -> Result<()> {
        repeat(
            0..,
            alt((
                string.void(),
                bracketed,
                comment,
                none_of(['(', ')', '[', ']', '{', '}', '"', '\'', '#']).void(),
            )),
        )
        .parse_next(input)
    };

    alt((
        delimited('(', contents, ')'),
        delimited('[', contents, ']'),
        delimited('{', contents, '}'),
    ))
    .parse_next(input)
}

fn string_literal(input: &mut &str) -> Result<String> {
    string.verify_map(|value| value).parse_next(input)
}

/// A quoted string, which is `None` when it's interpolated.
fn string(input: &mut &str) -> Result<Option<String>> {
    let single = delimited(
        '\'',
        repeat(0.., alt((preceded('\\', any), none_of(['\'', '\\'])))).fold(
            String::new,
            |mut value, c| {
                value.push(c);
                value
            },
        ),
        '\'',
    )
    .map(Some);

    let double = delimited(
        '"',
        repeat(
            0..,
            alt((
                preceded('\\', any).map(Some),
                preceded(('#', peek('{')), bracketed).map(|()| None),
                none_of(['"', '\\']).map(Some),
            )),
        )
        .fold(
            || Some(String::new()),
            |value: Option<String>, c| {
                let (mut value, c) = (value?, c?);
                value.push(c);
                Some(value)
            },
        ),
        '"',
    );

    alt((single, double, percent_string)).parse_next(input)
}

/// A `%q` string, like the `%q<rake>` in generated gemspecs, or an `%Q` string,
/// which is interpolated. Bracket delimiters nest, like `%q(a (b) c)`.
fn percent_string(input: &mut &str) -> Result<Option<String>> {
    let interpolates = preceded('%', alt(('q'.value(false), 'Q'.value(true)))).parse_next(input)?;
    let open = one_of(|c: char| !c.is_alphanumeric() && !c.is_whitespace()).parse_next(input)?;
    let close = match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        other => other,
    };

    let mut value = Some(String::new());
    let mut depth = 0;

    loop {
        let c = any.parse_next(input)?;

        let c = match c {
            '\\' => any.parse_next(input)?,
            '#' if interpolates && input.starts_with('{') => {
                bracketed.parse_next(input)?;
                value = None;
                continue;
            }
            c if c == close && depth == 0 => break,
            c if c == close => {
                depth -= 1;
                c
            }
            c if c == open => {
                depth += 1;
                c
            }
            c => c,
        };

        if let Some(value) = &mut value {
            value.push(c);
        }
    }

    Ok(value)
}

fn is_word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn ident<'i>(input: &mut &'i str) -> Result<&'i str> {
    (
        one_of(is_word_start),
        take_while(0.., |c: char| c.is_ascii_alphanumeric() || c == '_'),
        opt(one_of(['?', '!'])),
    )
        .take()
        .parse_next(input)
}

fn keyword<'i>(word: &'static str) -> impl Parser<&'i str, &'i str, winnow::error::ContextError> {
    terminated(
        word,
        not(one_of(|c: char| c.is_ascii_alphanumeric() || c == '_')),
    )
}

/// Words that can't start a method call.
fn reserved<'i>(input: &mut &'i str) -> Result<&'i str> {
    let keywords = [
        "else", "elsif", "when", "in", "ensure", "rescue", "case", "while", "until", "for",
        "begin", "def", "class", "module",
    ];

    alt((stop_keyword, alt(keywords.map(keyword)))).parse_next(input)
}

/// Words that end an argument.
fn stop_keyword<'i>(input: &mut &'i str) -> Result<&'i str> {
    alt(["do", "end", "if", "unless", "then"].map(keyword)).parse_next(input)
}

fn comment(input: &mut &str) -> Result<()> {
    ('#', till_line_ending).void().parse_next(input)
}

/// Spaces and tabs, including `\` line continuations.
fn ws(input: &mut &str) -> Result<()> {
    repeat(
        0..,
        alt((one_of([' ', '\t']).void(), ('\\', line_ending).void())),
    )
    .parse_next(input)
}

/// Whitespace, newlines and comments, like within brackets.
fn ws_nl(input: &mut &str) -> Result<()> {
    repeat(
        0..,
        alt((
            one_of([' ', '\t', '\r', '\n']).void(),
            ('\\', line_ending).void(),
            comment,
        )),
    )
    .parse_next(input)
}

/// Whitespace and comments between statements.
fn separators(input: &mut &str) -> Result<()> {
    repeat(0.., alt((one_of([';']).void(), ws_nl1))).parse_next(input)
}

fn ws_nl1(input: &mut &str) -> Result<()> {
    alt((
        one_of([' ', '\t', '\r', '\n']).void(),
        ('\\', line_ending).void(),
        comment,
    ))
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str) -> Call {
        Call {
            name: name.into(),
            ..Default::default()
        }
    }

    fn string(value: &str) -> Value {
        Value::String(value.into())
    }

    fn symbol(value: &str) -> Value {
        Value::Symbol(value.into())
    }

    #[test]
    fn parses_calls_with_every_option_style() {
        let statements = parse(
            r#"
gem "rails", ">= 7.1", "< 8"
gem 'support', :path => '../libs/support', "require" => false
gem("billing", path: "../libs/billing", groups: [:test, "ci"],)
"#,
        );

        assert_eq!(
            statements,
            [
                Statement::Call(Call {
                    args: vec![string("rails"), string(">= 7.1"), string("< 8")],
                    ..call("gem")
                }),
                Statement::Call(Call {
                    args: vec![string("support")],
                    options: vec![
                        ("path".into(), string("../libs/support")),
                        ("require".into(), Value::Bool(false)),
                    ],
                    ..call("gem")
                }),
                Statement::Call(Call {
                    args: vec![string("billing")],
                    options: vec![
                        ("path".into(), string("../libs/billing")),
                        (
                            "groups".into(),
                            Value::Array(vec![symbol("test"), string("ci")])
                        ),
                    ],
                    ..call("gem")
                }),
            ]
        );
    }

    #[test]
    fn parses_blocks_and_assignments() {
        let statements = parse(
            r#"
Gem::Specification.new do |spec|
  spec.name = "billing".freeze
  spec.add_dependency "core",
    "~> 1.0"
end
group :test do gem "rspec" end
"#,
        );

        assert_eq!(
            statements,
            [
                Statement::Call(Call {
                    receiver: Some("Gem::Specification".into()),
                    block: Some(vec![
                        Statement::Assign {
                            receiver: Some("spec".into()),
                            name: "name".into(),
                            value: string("billing"),
                        },
                        Statement::Call(Call {
                            receiver: Some("spec".into()),
                            args: vec![string("core"), string("~> 1.0")],
                            ..call("add_dependency")
                        }),
                    ]),
                    ..call("new")
                }),
                Statement::Call(Call {
                    args: vec![symbol("test")],
                    block: Some(vec![Statement::Call(Call {
                        args: vec![string("rspec")],
                        ..call("gem")
                    })]),
                    ..call("group")
                }),
            ]
        );
    }

    #[test]
    fn keeps_unsupported_expressions_as_other_values() {
        let statements = parse(
            r#"
gem "dynamic", path: File.expand_path("../libs/dynamic", __dir__)
gem "edge", github: "rails/#{repo}" if ENV["EDGE"]
install_if -> { RUBY_PLATFORM =~ /darwin/ } do
  gem "rb-fsevent"
end
"#,
        );

        assert_eq!(
            statements,
            [
                Statement::Call(Call {
                    args: vec![string("dynamic")],
                    options: vec![("path".into(), Value::Other)],
                    ..call("gem")
                }),
                Statement::Call(Call {
                    args: vec![string("edge")],
                    options: vec![("github".into(), Value::Other)],
                    ..call("gem")
                }),
                Statement::Call(Call {
                    args: vec![Value::Other],
                    block: Some(vec![Statement::Call(Call {
                        args: vec![string("rb-fsevent")],
                        ..call("gem")
                    })]),
                    ..call("install_if")
                }),
            ]
        );
    }

    #[test]
    fn keeps_statements_in_unsupported_constructs() {
        let statements = parse(
            r#"
if ENV["EDGE"]
  gem "rails", github: "rails/rails"
else
  gem "rails"
end
%w[a b].each do |name|
  gem name
end
"#,
        );

        assert_eq!(
            statements,
            [
                Statement::Other(vec![
                    Statement::Call(Call {
                        args: vec![string("rails")],
                        options: vec![("github".into(), string("rails/rails"))],
                        ..call("gem")
                    }),
                    Statement::Other(vec![]),
                    Statement::Call(Call {
                        args: vec![string("rails")],
                        ..call("gem")
                    }),
                ]),
                Statement::Other(vec![Statement::Call(Call {
                    args: vec![Value::Other],
                    ..call("gem")
                })]),
            ]
        );
    }

    #[test]
    fn parses_percent_strings() {
        let statements = parse(
            r#"
s.add_runtime_dependency(%q<irb>.freeze, ["~> 1.10".freeze])
s.summary = %q(Nested (parens) and \) escapes)
s.description = %Q{#{name} is interpolated}
"#,
        );

        assert_eq!(
            statements,
            [
                Statement::Call(Call {
                    receiver: Some("s".into()),
                    args: vec![string("irb"), Value::Array(vec![string("~> 1.10")])],
                    ..call("add_runtime_dependency")
                }),
                Statement::Assign {
                    receiver: Some("s".into()),
                    name: "summary".into(),
                    value: string("Nested (parens) and ) escapes"),
                },
                Statement::Assign {
                    receiver: Some("s".into()),
                    name: "description".into(),
                    value: Value::Other,
                },
            ]
        );
    }

    #[test]
    fn recovers_from_unbalanced_input() {
        let statements = parse(
            r#"
end
gem "a", path: ("unclosed"
gem "b"
"#,
        );

        assert_eq!(
            statements.last(),
            Some(&Statement::Call(Call {
                args: vec![string("b")],
                ..call("gem")
            }))
        );
    }
}
