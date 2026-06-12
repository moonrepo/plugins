//! Best-effort `Gemfile` / `*.gemspec` parser.
//!
//! A `Gemfile` is executable Ruby script, so this is a tolerant
//! line scanner, not an interpreter: it recognizes the common declarative
//! shapes and silently ignores anything dynamic (loops, conditionals, eval).
//! `Gemfile.lock` remains the source of truth for correctness-sensitive data;
//! this parse drives human-facing manifest info and the `path:`-gem project
//! graph.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Runtime,
    Development,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemDecl {
    pub name: String,
    pub requirement: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub scope: Scope,
}

/// Parse `gem` declarations from a `Gemfile`, tracking `group` blocks so
/// dev/test gems land in [`Scope::Development`].
pub fn parse_gemfile(content: &str) -> Vec<GemDecl> {
    let mut decls = vec![];
    let mut group_stack: Vec<Scope> = vec![];

    for line in content.lines() {
        let line = strip_comment(line);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // `group :development, :test do` ... `end`
        if let Some(rest) = line.strip_prefix("group ") {
            if line.ends_with("do") {
                group_stack.push(scope_from_groups(rest));
            }
            continue;
        }
        if line == "end" {
            group_stack.pop();
            continue;
        }

        if let Some(rest) = line.strip_prefix("gem ")
            && let Some(decl) = parse_gem_line(rest, current_scope(&group_stack))
        {
            decls.push(decl);
        }
    }

    decls
}

/// A parsed `*.gemspec`: the gem it provides plus its declared dependencies.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Gemspec {
    pub name: Option<String>,
    pub version: Option<String>,
    pub dependencies: Vec<GemDecl>,
}

/// Parse the common `add_dependency` / `add_development_dependency` and
/// `name`/`version` assignments from a `*.gemspec`.
pub fn parse_gemspec(content: &str) -> Gemspec {
    let mut spec = Gemspec::default();

    for line in content.lines() {
        let line = strip_comment(line);
        let line = line.trim();

        if let Some(rest) = after_assignment(line, "name") {
            spec.name = Some(unquote(rest));
        } else if let Some(rest) = after_assignment(line, "version") {
            spec.version = Some(unquote(rest));
        } else if let Some(args) = after_call(line, "add_development_dependency")
            && let Some(decl) = parse_gem_line(args, Scope::Development)
        {
            spec.dependencies.push(decl);
        } else if let Some(args) = after_call(line, "add_dependency")
            .or_else(|| after_call(line, "add_runtime_dependency"))
            && let Some(decl) = parse_gem_line(args, Scope::Runtime)
        {
            spec.dependencies.push(decl);
        }
    }

    spec
}

fn current_scope(stack: &[Scope]) -> Scope {
    // Any enclosing dev/test group makes the gem a dev dependency.
    if stack.contains(&Scope::Development) {
        Scope::Development
    } else {
        Scope::Runtime
    }
}

fn scope_from_groups(s: &str) -> Scope {
    if s.contains("development") || s.contains("test") {
        Scope::Development
    } else {
        Scope::Runtime
    }
}

/// Parse the argument list of a `gem`/`add_*` call (everything after the
/// keyword), e.g. `"rails", "~> 7.1"` or `"x", path: "../x"`.
fn parse_gem_line(args: &str, default_scope: Scope) -> Option<GemDecl> {
    let mut name: Option<String> = None;
    let mut requirements = vec![];
    let mut path = None;
    let mut git = None;
    let mut scope = default_scope;

    for part in split_args(args) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some((key, val)) = split_option(part) {
            let val = unquote(val.trim());
            match key {
                "path" => path = Some(val),
                "git" | "github" => git = Some(val),
                "group" => scope = scope_from_groups(&val),
                _ => {} // require:, ref:, tag:, branch:, platforms: — ignored
            }
        } else if is_quoted(part) {
            let value = unquote(part);
            if name.is_none() {
                name = Some(value);
            } else {
                requirements.push(value);
            }
        }
    }

    let name = name?;
    if name.is_empty() {
        return None;
    }

    Some(GemDecl {
        name,
        requirement: (!requirements.is_empty()).then(|| requirements.join(", ")),
        path,
        git,
        scope,
    })
}

/// Split a comma-separated argument list. Naive (doesn't account for commas
/// inside `[...]` arrays); acceptable for a best-effort scanner.
fn split_args(args: &str) -> Vec<&str> {
    args.split(',').collect()
}

/// Recognize a `key: value` option token, returning `(key, value)`. Requires
/// the key to be a bare identifier so we don't mistake a quoted version
/// requirement (which may contain `:`) for an option.
fn split_option(part: &str) -> Option<(&str, &str)> {
    let (key, val) = part.split_once(':')?;
    let key = key.trim();
    if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Some((key, val))
    } else {
        None
    }
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn is_quoted(s: &str) -> bool {
    let s = s.trim();
    (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches(|c| c == '"' || c == '\'').to_string()
}

/// Return the value of `<target> = <value>` / `<recv>.<target> = <value>`.
fn after_assignment<'a>(line: &'a str, target: &str) -> Option<&'a str> {
    let (lhs, rhs) = line.split_once('=')?;
    let lhs = lhs.trim();
    let attr = lhs.rsplit('.').next().unwrap_or(lhs).trim();
    (attr == target).then_some(rhs.trim())
}

/// Return the argument list of a `<recv>.<name>(<args>)` / `<recv>.<name> <args>` call.
fn after_call<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let idx = line.find(name)?;
    // Ensure it's a call boundary (preceded by start or `.`/space).
    let before_ok = line[..idx]
        .chars()
        .last()
        .map(|c| c == '.' || c.is_whitespace())
        .unwrap_or(true);
    if !before_ok {
        return None;
    }

    let rest = line[idx + name.len()..].trim_start();
    let rest = rest.strip_prefix('(').unwrap_or(rest);
    Some(rest.trim_end_matches(')').trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_gems_and_requirements() {
        let decls = parse_gemfile(
            r#"
source "https://rubygems.org"
gem "rails", "~> 7.1"
gem "pg"
"#,
        );

        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0].name, "rails");
        assert_eq!(decls[0].requirement.as_deref(), Some("~> 7.1"));
        assert_eq!(decls[0].scope, Scope::Runtime);
        assert_eq!(decls[1].name, "pg");
        assert_eq!(decls[1].requirement, None);
    }

    #[test]
    fn tracks_group_blocks_as_dev_scope() {
        let decls = parse_gemfile(
            r#"
gem "rails"
group :development, :test do
  gem "rspec"
end
gem "puma"
"#,
        );

        let rspec = decls.iter().find(|d| d.name == "rspec").unwrap();
        assert_eq!(rspec.scope, Scope::Development);
        // The group is popped at `end`; puma is back to runtime.
        let puma = decls.iter().find(|d| d.name == "puma").unwrap();
        assert_eq!(puma.scope, Scope::Runtime);
    }

    #[test]
    fn handles_inline_group_option() {
        let decls = parse_gemfile(r#"gem "rubocop", require: false, group: :development"#);
        assert_eq!(decls[0].scope, Scope::Development);
    }

    #[test]
    fn captures_path_and_git_sources() {
        let decls = parse_gemfile(
            r#"
gem "billing", path: "../libs/billing"
gem "some_fork", git: "https://github.com/me/some_fork.git", branch: "main"
"#,
        );

        let billing = decls.iter().find(|d| d.name == "billing").unwrap();
        assert_eq!(billing.path.as_deref(), Some("../libs/billing"));

        let fork = decls.iter().find(|d| d.name == "some_fork").unwrap();
        assert_eq!(
            fork.git.as_deref(),
            Some("https://github.com/me/some_fork.git")
        );
    }

    #[test]
    fn ignores_comments() {
        let decls = parse_gemfile(
            r#"
# gem "ignored"
gem "real" # trailing comment
"#,
        );
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].name, "real");
    }

    #[test]
    fn parses_gemspec_dependencies() {
        let spec = parse_gemspec(
            r#"
Gem::Specification.new do |s|
  s.name = "billing"
  s.version = "0.1.0"
  s.add_dependency "activesupport", ">= 7.0"
  s.add_development_dependency "rspec"
end
"#,
        );

        assert_eq!(spec.name.as_deref(), Some("billing"));
        assert_eq!(spec.version.as_deref(), Some("0.1.0"));
        assert_eq!(spec.dependencies.len(), 2);

        let active = spec
            .dependencies
            .iter()
            .find(|d| d.name == "activesupport")
            .unwrap();
        assert_eq!(active.scope, Scope::Runtime);
        assert_eq!(active.requirement.as_deref(), Some(">= 7.0"));

        let rspec = spec
            .dependencies
            .iter()
            .find(|d| d.name == "rspec")
            .unwrap();
        assert_eq!(rspec.scope, Scope::Development);
    }
}
