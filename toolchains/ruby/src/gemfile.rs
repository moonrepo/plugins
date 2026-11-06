//! Best-effort `Gemfile` and gemspec scanner.
//!
//! Both are executable Ruby script, so this intentionally does not try to
//! evaluate them. It only extracts literal gem declarations, so moon can infer
//! project-to-project edges and hash gem versions in Ruby monorepos.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gem {
    pub name: String,
    /// Directory of a local gem.
    pub path: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Gemspec {
    pub name: Option<String>,
    pub dependencies: Vec<GemspecDependency>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GemspecDependency {
    pub name: String,
    pub development: bool,
}

enum Block {
    Group(Vec<String>),
    Path(String),
    Other,
}

/// Parse literal gem declarations from a `Gemfile`, preserving Bundler group
/// names verbatim. The caller decides how to map groups onto moon scopes.
/// Local gems are declared with `path:`, `path "<dir>" do` blocks, and
/// `gemspec path:`.
pub fn parse_gems(content: &str) -> Vec<Gem> {
    let mut gems = vec![];
    let mut blocks: Vec<Block> = vec![];

    for line in join_continued_lines(content) {
        if line == "end" {
            blocks.pop();
            continue;
        }

        // Keep non-group blocks on the stack too, so that their
        // `end` doesn't accidentally pop an enclosing group
        if let Some(call) = strip_block_opener(&line) {
            blocks.push(if let Some(args) = after_bare_call(call, "group") {
                Block::Group(parse_groups(args))
            } else if let Some(args) = after_bare_call(call, "path")
                && let Some(dir) = split_args(args).first().copied().and_then(quoted)
            {
                Block::Path(dir.to_owned())
            } else {
                Block::Other
            });
            continue;
        }

        if starts_ruby_block(&line) {
            blocks.push(Block::Other);
            continue;
        }

        let gem = if let Some(args) = after_bare_call(&line, "gem") {
            let block_dir = blocks.iter().rev().find_map(|block| match block {
                Block::Path(dir) => Some(dir.as_str()),
                _ => None,
            });

            parse_gem_call(args, block_dir)
        } else if let Some(args) = after_bare_call(&line, "gemspec") {
            parse_gemspec_call(args)
        } else {
            None
        };

        if let Some(mut gem) = gem {
            for block in &blocks {
                if let Block::Group(groups) = block {
                    for group in groups {
                        if !gem.groups.contains(group) {
                            gem.groups.push(group.clone());
                        }
                    }
                }
            }

            gems.push(gem);
        }
    }

    gems
}

/// Strip comments, and join calls that continue onto the next line
/// (a trailing `,` or `\`) into a single line.
fn join_continued_lines(content: &str) -> Vec<String> {
    let mut lines = vec![];
    let mut current = String::new();

    for line in content.lines() {
        let line = strip_comment(line).trim();

        if line.is_empty() {
            continue;
        }

        if !current.is_empty() {
            current.push(' ');
        }

        current.push_str(line.trim_end_matches('\\').trim_end());

        if !line.ends_with(',') && !line.ends_with('\\') {
            lines.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

/// Return the call before a trailing `do` or `do |args|`.
fn strip_block_opener(line: &str) -> Option<&str> {
    let call = match line.strip_suffix('|') {
        Some(rest) => rest.rsplit_once(" do |")?.0,
        None => line.strip_suffix(" do")?,
    };

    Some(call.trim_end())
}

fn parse_gem_call(args: &str, block_dir: Option<&str>) -> Option<Gem> {
    let mut name = None;
    let mut path = None;
    let mut groups = vec![];

    for part in split_args(args) {
        let part = part.trim();

        if name.is_none()
            && let Some(value) = quoted(part)
        {
            name = Some(value.to_owned());
            continue;
        }

        if let Some((key, value)) = split_option(part) {
            match key {
                "path" => path = quoted(value.trim()).map(str::to_owned),
                "group" | "groups" => {
                    for group in parse_groups(value) {
                        if !groups.contains(&group) {
                            groups.push(group);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let name = name?;

    // Bundler finds gems in a path block by their gemspec name,
    // which is typically a directory of the same name
    let path =
        path.or_else(|| block_dir.map(|dir| format!("{}/{name}", dir.trim_end_matches('/'))));

    Some(Gem { name, path, groups })
}

/// A bare `gemspec` is the current project, so only `gemspec path:`
/// references another gem.
fn parse_gemspec_call(args: &str) -> Option<Gem> {
    let mut name = None;
    let mut path = None;

    for part in split_args(args) {
        if let Some((key, value)) = split_option(part.trim()) {
            match key {
                "name" => name = quoted(value).map(str::to_owned),
                "path" => path = quoted(value).map(str::to_owned),
                _ => {}
            }
        }
    }

    let path = path?;
    let name = name.unwrap_or_else(|| {
        path.trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned()
    });

    Some(Gem {
        name,
        path: Some(path),
        groups: vec![],
    })
}

/// Parse the literal name and dependencies from a gemspec, which are
/// set on the block variable, like `spec.add_dependency "billing"`.
pub fn parse_gemspec(content: &str) -> Gemspec {
    let mut gemspec = Gemspec::default();

    for line in join_continued_lines(content) {
        let Some((var, call)) = line.split_once('.') else {
            continue;
        };

        if var.is_empty() || !var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }

        if let Some(value) = call
            .strip_prefix("name")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .filter(|value| !value.starts_with('='))
        {
            gemspec.name = quoted(value.trim().trim_end_matches(".freeze")).map(str::to_owned);
            continue;
        }

        for (method, development) in [
            ("add_dependency", false),
            ("add_runtime_dependency", false),
            ("add_development_dependency", true),
        ] {
            if let Some(args) = after_bare_call(call, method)
                && let Some(name) = split_args(args).first().copied().and_then(quoted)
            {
                gemspec.dependencies.push(GemspecDependency {
                    name: name.to_owned(),
                    development,
                });
            }
        }
    }

    gemspec
}

fn parse_groups(args: &str) -> Vec<String> {
    let args = args.trim();
    let args = args
        .strip_prefix('[')
        .and_then(|args| args.strip_suffix(']'))
        .unwrap_or(args);

    split_args(args)
        .into_iter()
        .filter_map(|part| group_name(part.trim()))
        .collect()
}

fn group_name(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches(" do").trim();

    if let Some(symbol) = value.strip_prefix(':') {
        let symbol = symbol.trim();
        if !symbol.is_empty()
            && symbol
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Some(symbol.to_owned());
        }
    }

    quoted(value).map(str::to_owned)
}

fn after_bare_call<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(name)?;

    if let Some(args) = rest.strip_prefix('(') {
        return Some(args.trim_end_matches(')').trim());
    }

    let first = rest.chars().next()?;
    first.is_whitespace().then(|| rest.trim())
}

/// Recognize `key: value` and `:key => value` option tokens.
fn split_option(part: &str) -> Option<(&str, &str)> {
    if let Some((key, val)) = part.split_once(':') {
        let key = key.trim();
        if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Some((key, val.trim()));
        }
    }

    if let Some((key, val)) = part.split_once("=>") {
        let key = key
            .trim()
            .trim_start_matches(':')
            .trim_matches(|c| c == '\'' || c == '"');
        if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Some((key, val.trim()));
        }
    }

    None
}

fn quoted(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.len() < 2 {
        return None;
    }

    let quote = value.as_bytes()[0] as char;
    if quote != '\'' && quote != '"' {
        return None;
    }

    value
        .strip_prefix(quote)
        .and_then(|value| value.strip_suffix(quote))
}

fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match (quote, ch) {
            (Some(_), '\\') => escaped = true,
            (Some(q), c) if c == q => quote = None,
            (None, '\'' | '"') => quote = Some(ch),
            (None, '#') => return &line[..idx],
            _ => {}
        }
    }

    line
}

fn starts_ruby_block(line: &str) -> bool {
    [
        "if ", "unless ", "case ", "begin", "while ", "until ", "for ", "def ",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn split_args(args: &str) -> Vec<&str> {
    let mut parts = vec![];
    let mut quote = None;
    let mut escaped = false;
    let mut depth = 0u32;
    let mut start = 0;

    for (idx, ch) in args.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match (quote, ch) {
            (Some(_), '\\') => escaped = true,
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(ch),
            (None, '[' | '(' | '{') => depth += 1,
            (None, ']' | ')' | '}') => depth = depth.saturating_sub(1),
            (None, ',') if depth == 0 => {
                parts.push(args[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(args[start..].trim());
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_path_gems(content: &str) -> Vec<Gem> {
        parse_gems(content)
            .into_iter()
            .filter(|gem| gem.path.is_some())
            .collect()
    }

    #[test]
    fn captures_literal_path_gems() {
        let gems = parse_path_gems(
            r#"
source "https://rubygems.org"
gem "rails"
gem "billing", path: "../libs/billing"
gem 'support', :path => '../libs/support'
"#,
        );

        assert_eq!(gems.len(), 2);
        assert_eq!(gems[0].name, "billing");
        assert_eq!(gems[0].path.as_deref(), Some("../libs/billing"));
        assert_eq!(gems[1].name, "support");
        assert_eq!(gems[1].path.as_deref(), Some("../libs/support"));
    }

    #[test]
    fn preserves_arbitrary_groups() {
        let gems = parse_path_gems(
            r#"
group :ci, :assets do
  gem "fixtures", path: "../libs/fixtures", groups: [:test, :docs]
end
gem "billing", path: "../libs/billing", group: :deploy
"#,
        );

        assert_eq!(gems[0].groups, ["test", "docs", "ci", "assets"]);
        assert_eq!(gems[1].groups, ["deploy"]);
    }

    #[test]
    fn captures_groups_with_bundler_options() {
        let gems = parse_path_gems(
            r#"
group :migrations, optional: true do
  gem "migrations-core", path: "migrations/core"
end
"#,
        );

        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].name, "migrations-core");
        assert_eq!(gems[0].path.as_deref(), Some("migrations/core"));
        assert_eq!(gems[0].groups, ["migrations"]);
    }

    #[test]
    fn captures_single_quoted_keyword_path_to_parent() {
        let gems = parse_path_gems("gem 'spree', path: '../'");

        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].name, "spree");
        assert_eq!(gems[0].path.as_deref(), Some("../"));
    }

    #[test]
    fn balances_non_group_blocks() {
        let gems = parse_path_gems(
            r#"
group :test do
  source "https://example.com" do
    gem "fixtures", path: "../libs/fixtures"
  end
  gem "support", path: "../libs/support"
end
"#,
        );

        assert_eq!(gems[0].groups, ["test"]);
        assert_eq!(gems[1].groups, ["test"]);
    }

    #[test]
    fn ignores_dynamic_paths() {
        let gems = parse_path_gems(
            r#"
gem "dynamic", path: File.expand_path("../libs/dynamic", __dir__)
"#,
        );

        assert!(gems.is_empty());
    }

    #[test]
    fn ignores_comments_outside_quotes() {
        let gems = parse_path_gems(
            r#"
# gem "ignored", path: "../ignored"
gem "real", path: "../libs/#real" # trailing comment
"#,
        );

        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].path.as_deref(), Some("../libs/#real"));
    }
    #[test]
    fn captures_multi_line_gem_calls() {
        let gems = parse_path_gems(
            r#"
gem "billing",
  path: "../libs/billing",
  groups: [:test,
    :ci]
"#,
        );

        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].path.as_deref(), Some("../libs/billing"));
        assert_eq!(gems[0].groups, ["test", "ci"]);
    }

    #[test]
    fn captures_gems_in_path_blocks() {
        let gems = parse_path_gems(
            r#"
path "components" do
  gem "admin_ui"
  gem "public_ui", path: "ui/public"
end
gem "rails"
"#,
        );

        assert_eq!(gems.len(), 2);
        assert_eq!(gems[0].name, "admin_ui");
        assert_eq!(gems[0].path.as_deref(), Some("components/admin_ui"));
        assert_eq!(gems[1].path.as_deref(), Some("ui/public"));
    }

    #[test]
    fn captures_gemspec_paths() {
        let gems = parse_path_gems(
            r#"
gemspec
gemspec path: "../libs/billing"
gemspec path: "../libs/core", name: "core-lib"
"#,
        );

        assert_eq!(gems.len(), 2);
        assert_eq!(gems[0].name, "billing");
        assert_eq!(gems[0].path.as_deref(), Some("../libs/billing"));
        assert_eq!(gems[1].name, "core-lib");
    }

    #[test]
    fn balances_blocks_with_arguments_and_methods() {
        let gems = parse_path_gems(
            r#"
group :test do
  def local(name)
    "../libs/#{name}"
  end
  %w[a b].each do |name|
    gem name
  end
  gem "fixtures", path: "../libs/fixtures"
end
gem "billing", path: "../libs/billing"
"#,
        );

        assert_eq!(gems[0].groups, ["test"]);
        assert!(gems[1].groups.is_empty());
    }

    #[test]
    fn captures_gems_without_paths() {
        let gems = parse_gems(
            r#"
gem "rails", "~> 7.1"
group :test do
  gem "rspec"
end
gemspec
"#,
        );

        assert_eq!(gems.len(), 2);
        assert_eq!(gems[0].name, "rails");
        assert_eq!(gems[0].path, None);
        assert_eq!(gems[1].name, "rspec");
        assert_eq!(gems[1].groups, ["test"]);
    }

    #[test]
    fn parses_gemspec_name_and_dependencies() {
        let gemspec = parse_gemspec(
            r#"
require_relative "lib/billing/version"

Gem::Specification.new do |spec|
  spec.name          = "billing"
  spec.version       = Billing::VERSION
  spec.namespace     = "ignored"

  spec.add_dependency "core", "~> 1.0"
  spec.add_runtime_dependency("money")
  spec.add_development_dependency "rspec",
    ">= 3"
  spec.add_dependency name_from_variable
end
"#,
        );

        assert_eq!(gemspec.name.as_deref(), Some("billing"));
        assert_eq!(
            gemspec.dependencies,
            [
                GemspecDependency {
                    name: "core".into(),
                    development: false,
                },
                GemspecDependency {
                    name: "money".into(),
                    development: false,
                },
                GemspecDependency {
                    name: "rspec".into(),
                    development: true,
                },
            ]
        );
    }

    #[test]
    fn parses_frozen_gemspec_names() {
        let gemspec = parse_gemspec(r#"s.name = "billing".freeze"#);

        assert_eq!(gemspec.name.as_deref(), Some("billing"));
    }
}
