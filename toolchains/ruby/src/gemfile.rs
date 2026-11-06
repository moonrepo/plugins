//! Best-effort `Gemfile` and gemspec scanner.
//!
//! Both are executable Ruby script, so this intentionally does not try to
//! evaluate them. It only extracts literal gem declarations, so moon can infer
//! project-to-project edges and hash gem versions in Ruby monorepos.

mod parser;

use parser::{Call, Statement, Value};

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

/// Parse literal gem declarations from a `Gemfile`, preserving Bundler group
/// names verbatim. The caller decides how to map groups onto moon scopes.
/// Local gems are declared with `path:`, `path "<dir>" do` blocks, and
/// `gemspec path:`.
pub fn parse_gems(content: &str) -> Vec<Gem> {
    let mut gems = vec![];

    collect_gems(&parser::parse(content), &Scope::default(), &mut gems);

    gems
}

/// The `group` and `path` blocks a gem is declared in.
#[derive(Clone, Default)]
struct Scope<'a> {
    groups: Vec<String>,
    path: Option<&'a str>,
}

fn collect_gems<'a>(statements: &'a [Statement], scope: &Scope<'a>, gems: &mut Vec<Gem>) {
    for statement in statements {
        let mut inner = scope.clone();

        // Gems in other blocks and conditionals are still declared
        let body = match statement {
            Statement::Call(call) => {
                if call.receiver.is_none() {
                    match call.name.as_str() {
                        "gem" => gems.extend(parse_gem_call(call, scope)),
                        "gemspec" => gems.extend(parse_gemspec_call(call)),
                        "group" => {
                            for arg in &call.args {
                                push_group_names(&mut inner.groups, arg);
                            }
                        }
                        "path" => inner.path = call.args.first().and_then(Value::as_string),
                        _ => {}
                    };
                }

                call.block.as_deref()
            }
            Statement::Other(body) => Some(body.as_slice()),
            Statement::Assign { .. } => None,
        };

        if let Some(body) = body {
            collect_gems(body, &inner, gems);
        }
    }
}

fn parse_gem_call(call: &Call, scope: &Scope) -> Option<Gem> {
    let name = call.args.first()?.as_string()?.to_owned();
    let mut groups = vec![];

    for key in ["group", "groups"] {
        if let Some(value) = call.get_option(key) {
            push_group_names(&mut groups, value);
        }
    }

    for group in &scope.groups {
        if !groups.contains(group) {
            groups.push(group.clone());
        }
    }

    // Bundler finds gems in a path block by their gemspec name,
    // which is typically a directory of the same name
    let path = call
        .get_option("path")
        .and_then(Value::as_string)
        .map(str::to_owned)
        .or_else(|| {
            scope
                .path
                .map(|dir| format!("{}/{name}", dir.trim_end_matches('/')))
        });

    Some(Gem { name, path, groups })
}

/// A bare `gemspec` is the current project, so only `gemspec path:`
/// references another gem.
fn parse_gemspec_call(call: &Call) -> Option<Gem> {
    let path = call.get_option("path")?.as_string()?.to_owned();
    let name = match call.get_option("name").and_then(Value::as_string) {
        Some(name) => name.to_owned(),
        None => path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned(),
    };

    Some(Gem {
        name,
        path: Some(path),
        groups: vec![],
    })
}

fn push_group_names(groups: &mut Vec<String>, value: &Value) {
    match value {
        Value::String(name) | Value::Symbol(name) => {
            if !groups.contains(name) {
                groups.push(name.clone());
            }
        }
        Value::Array(values) => {
            for value in values {
                push_group_names(groups, value);
            }
        }
        _ => {}
    }
}

/// Parse the literal name and dependencies from a gemspec, which are
/// set on the block variable, like `spec.add_dependency "billing"`.
pub fn parse_gemspec(content: &str) -> Gemspec {
    let mut gemspec = Gemspec::default();

    collect_gemspec(&parser::parse(content), &mut gemspec);

    gemspec
}

fn collect_gemspec(statements: &[Statement], gemspec: &mut Gemspec) {
    for statement in statements {
        match statement {
            Statement::Assign {
                receiver: Some(_),
                name,
                value: Value::String(value),
            } if name == "name" => {
                gemspec.name = Some(value.clone());
            }
            Statement::Call(call) => {
                let development = match call.name.as_str() {
                    "add_dependency" | "add_runtime_dependency" => Some(false),
                    "add_development_dependency" => Some(true),
                    _ => None,
                };

                // Older generated gemspecs repeat dependencies in
                // fallback branches, so the first one wins
                if call.receiver.is_some()
                    && let Some(development) = development
                    && let Some(name) = call.args.first().and_then(Value::as_string)
                    && !gemspec.dependencies.iter().any(|dep| dep.name == name)
                {
                    gemspec.dependencies.push(GemspecDependency {
                        name: name.to_owned(),
                        development,
                    });
                }

                if let Some(body) = &call.block {
                    collect_gemspec(body, gemspec);
                }
            }
            Statement::Other(body) => collect_gemspec(body, gemspec),
            Statement::Assign { .. } => {}
        }
    }
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
    fn captures_gems_in_conditionals_and_one_line_blocks() {
        let gems = parse_path_gems(
            r#"
if ENV["EDGE"]
  gem "edge", path: "../edge"
end
gem "modifier", path: "../modifier" if ENV["MODIFIER"]
group :test do gem "inline", path: "../inline" end
install_if -> { RUBY_PLATFORM =~ /darwin/ } do
  gem "mac", path: "../mac"
end
"#,
        );

        assert_eq!(
            gems.iter().map(|gem| gem.name.as_str()).collect::<Vec<_>>(),
            ["edge", "modifier", "inline", "mac"]
        );
        assert_eq!(gems[2].groups, ["test"]);
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
    fn parses_generated_gemspecs() {
        let gemspec = parse_gemspec(
            r#"
# -*- encoding: utf-8 -*-
Gem::Specification.new do |s|
  s.name = "billing".freeze
  s.summary = %q{Bills things}
  s.add_runtime_dependency(%q<core>.freeze, [">= 0".freeze])

  if s.respond_to? :add_runtime_dependency then
    s.add_development_dependency(%q<rspec>.freeze, ["~> 3".freeze])
  else
    s.add_dependency(%q<rspec>.freeze, ["~> 3".freeze])
  end
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
