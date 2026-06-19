//! Best-effort `Gemfile` path-gem scanner.
//!
//! A `Gemfile` is executable Ruby script, so this intentionally does not try to
//! parse dependencies in general. It only extracts literal `gem ..., path: ...`
//! declarations so moon can infer project-to-project edges in Ruby monorepos.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathGem {
    pub name: String,
    pub path: String,
    pub groups: Vec<String>,
}

/// Parse literal `path:` gem declarations from a `Gemfile`, preserving Bundler
/// group names verbatim. The caller decides how to map groups onto moon scopes.
pub fn parse_path_gems(content: &str) -> Vec<PathGem> {
    let mut gems = vec![];
    let mut block_stack: Vec<Vec<String>> = vec![];

    for line in content.lines() {
        let line = strip_comment(line).trim().to_owned();
        if line.is_empty() {
            continue;
        }

        if line == "end" {
            block_stack.pop();
            continue;
        }

        if line.ends_with(" do") {
            if let Some(groups) = parse_group_call(&line) {
                block_stack.push(groups);
            } else {
                // Keep nested non-group blocks balanced so their `end` doesn't
                // accidentally pop an enclosing group.
                block_stack.push(vec![]);
            }
            continue;
        }

        if starts_ruby_block(&line) {
            block_stack.push(vec![]);
            continue;
        }

        if let Some(args) = after_bare_call(&line, "gem")
            && let Some(mut gem) = parse_gem_call(args)
        {
            for groups in &block_stack {
                for group in groups {
                    if !gem.groups.contains(group) {
                        gem.groups.push(group.clone());
                    }
                }
            }

            gems.push(gem);
        }
    }

    gems
}

fn parse_group_call(line: &str) -> Option<Vec<String>> {
    let line = line.strip_suffix(" do")?.trim_end();
    let args = after_bare_call(line, "group")?;
    Some(parse_groups(args))
}

fn parse_gem_call(args: &str) -> Option<PathGem> {
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

    Some(PathGem {
        name: name?,
        path: path?,
        groups,
    })
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
        "if ", "unless ", "case ", "begin", "while ", "until ", "for ",
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
        assert_eq!(gems[0].path, "../libs/billing");
        assert_eq!(gems[1].name, "support");
        assert_eq!(gems[1].path, "../libs/support");
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
        assert_eq!(gems[0].path, "migrations/core");
        assert_eq!(gems[0].groups, ["migrations"]);
    }

    #[test]
    fn captures_single_quoted_keyword_path_to_parent() {
        let gems = parse_path_gems("gem 'spree', path: '../'");

        assert_eq!(gems.len(), 1);
        assert_eq!(gems[0].name, "spree");
        assert_eq!(gems[0].path, "../");
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
        assert_eq!(gems[0].path, "../libs/#real");
    }
}
