//! Which task ids are already claimed by moon's inherited task files.
//!
//! Task inference must never contribute an id an inherited task file defines.
//! moon replaces a project-level task wholesale, but *merges* over an inherited
//! one with args appended — so an inferred `dotnet run` landing on an inherited
//! `echo inherited-run` produces `dotnet inherited-run run`. Yielding the id
//! entirely is the only safe option.

use moon_pdk_api::VirtualPath;
use starbase_utils::{fs, yaml};
use std::collections::BTreeMap;

/// Partial shape of an inherited tasks file (`.moon/tasks.yml` or
/// `.moon/tasks/**/*.yml`) — just enough to know which task ids it defines
/// and whether it can apply to dotnet projects.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InheritedTasksFile {
    inherited_by: Option<InheritedByScope>,
    tasks: BTreeMap<String, serde::de::IgnoredAny>,
}

/// The task ids alone, for when the scope cannot be understood. See
/// [`load_inherited_task_ids`] for why that case still reserves them.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct TaskIdsOnly {
    tasks: BTreeMap<String, serde::de::IgnoredAny>,
}

/// The `inheritedBy` conditions that can rule a file out for dotnet.
///
/// Only these two are read; `files`, `layers`, `stacks` and `tags` cannot
/// exclude a dotnet project on their own, so a file scoped by them alone is
/// treated as applying.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InheritedByScope {
    /// moon aliases this to the singular `toolchain`.
    #[serde(alias = "toolchain")]
    toolchains: Option<Condition>,

    /// moon aliases this to the singular `language`.
    #[serde(alias = "language")]
    languages: Option<Condition>,
}

/// One `inheritedBy` condition, mirroring moon's `InheritedConditionConfig`:
/// a single value, a list, or a clause of logical operators.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Condition {
    One(String),
    Many(Vec<String>),
    Clause(ConditionClause),
}

/// moon's `InheritedClauseConfig`. Each operator is itself one-or-many.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
struct ConditionClause {
    and: Option<OneOrMany>,
    or: Option<OneOrMany>,
    not: Option<OneOrMany>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn names(&self) -> &[String] {
        match self {
            Self::One(value) => std::slice::from_ref(value),
            Self::Many(values) => values,
        }
    }
}

impl Condition {
    /// Could this condition admit a dotnet project, given a predicate that
    /// recognizes dotnet identifiers in this condition's vocabulary?
    fn admits(&self, is_dotnet: &dyn Fn(&str) -> bool) -> bool {
        match self {
            Self::One(value) => is_dotnet(value),
            Self::Many(values) => values.iter().any(|value| is_dotnet(value)),
            Self::Clause(clause) => clause.admits(is_dotnet),
        }
    }
}

impl ConditionClause {
    fn admits(&self, is_dotnet: &dyn Fn(&str) -> bool) -> bool {
        // moon treats a clause with no operators as matching nothing, so there
        // is no inherited task to yield to.
        if self.and.is_none() && self.or.is_none() && self.not.is_none() {
            return false;
        }

        // Explicitly excluded.
        if self
            .not
            .as_ref()
            .is_some_and(|not| not.names().iter().any(|name| is_dotnet(name)))
        {
            return false;
        }

        match (&self.and, &self.or) {
            // Only `not`, and it did not name us: everything else is admitted.
            (None, None) => true,
            // A positive list has to name us. `and` is treated like `or` on
            // purpose — a project can carry several toolchains, so `and` naming
            // dotnet alongside others may still include dotnet projects.
            (and, or) => and
                .iter()
                .chain(or.iter())
                .any(|list| list.names().iter().any(|name| is_dotnet(name))),
        }
    }
}

fn is_dotnet_toolchain(id: &str) -> bool {
    id.eq_ignore_ascii_case("dotnet")
}

/// Recognize a dotnet `language` value. `dotnet`/`.net` and `csharp`/`c#` are
/// moon's own `LanguageType` variants and aliases; the rest reach moon as
/// `LanguageType::Other` but are what someone would plausibly write.
fn is_dotnet_language(language: &str) -> bool {
    matches!(
        language.to_lowercase().as_str(),
        "dotnet" | ".net" | "csharp" | "c#" | "fsharp" | "f#" | "vb" | "visualbasic"
    )
}

/// Can an inherited tasks file apply to dotnet projects? Only an explicit
/// `inheritedBy` scope naming other toolchains/languages rules it out;
/// everything else (unscoped, tag/stack/layer-scoped) is conservatively
/// assumed to apply — suppressing an inferred task is recoverable, while
/// moon's args-append merge of an inferred task over an inherited one
/// produces garbage commands.
fn applies_to_dotnet(scope: Option<&InheritedByScope>) -> bool {
    let Some(scope) = scope else {
        return true;
    };

    let mut scoped = false;

    if let Some(toolchains) = &scope.toolchains {
        scoped = true;

        if toolchains.admits(&is_dotnet_toolchain) {
            return true;
        }
    }

    if let Some(languages) = &scope.languages {
        scoped = true;

        if languages.admits(&is_dotnet_language) {
            return true;
        }
    }

    !scoped
}

fn collect_yaml_files(dir: &VirtualPath, out: &mut Vec<VirtualPath>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };

            let path = dir.join(&name);

            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                collect_yaml_files(&path, out);
            } else if name.ends_with(".yml") || name.ends_with(".yaml") {
                out.push(path);
            }
        }
    }
}

/// Task ids defined in inherited task files that can apply to dotnet
/// projects, mapped to the file that defines each one (for reporting).
/// Inference must never contribute one of these ids — see
/// `applies_to_dotnet` for why.
pub fn load_inherited_task_ids(workspace_root: &VirtualPath) -> BTreeMap<String, String> {
    let mut ids = BTreeMap::new();
    let mut files = vec![workspace_root.join(".moon").join("tasks.yml")];

    collect_yaml_files(&workspace_root.join(".moon").join("tasks"), &mut files);

    for file in files {
        if !file.exists() {
            continue;
        }

        let label = file.to_string();

        // A file whose `inheritedBy` this plugin cannot model is still a file
        // moon reads perfectly well, so its task ids must be reserved anyway.
        // Failing the other way — treating it as defining nothing — lets
        // inference contribute an id the file already defines, and moon merges
        // inherited and plugin tasks by *appending args*, producing a broken
        // command. Over-reserving only costs an inferred task.
        let claimed = match yaml::read_file::<InheritedTasksFile>(&file) {
            Ok(parsed) => applies_to_dotnet(parsed.inherited_by.as_ref())
                .then(|| parsed.tasks.into_keys().collect::<Vec<_>>()),
            Err(_) => match yaml::read_file::<TaskIdsOnly>(&file) {
                Ok(parsed) => Some(parsed.tasks.into_keys().collect()),
                // Not a tasks file at all, or unreadable: moon's problem to
                // report, and there is no id to yield to.
                Err(_) => None,
            },
        };

        for id in claimed.unwrap_or_default() {
            ids.entry(id).or_insert_with(|| label.clone());
        }
    }

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(toolchains: Option<&[&str]>, languages: Option<&[&str]>) -> InheritedByScope {
        let many = |list: &[&str]| {
            Condition::Many(list.iter().map(|id| id.to_string()).collect::<Vec<_>>())
        };

        InheritedByScope {
            toolchains: toolchains.map(many),
            languages: languages.map(many),
        }
    }

    /// Parse an `inheritedBy` block the way a real file would be parsed, so
    /// the tests cover deserialization and not just the matching logic.
    fn applies(yaml: &str) -> bool {
        let file: InheritedTasksFile = yaml::parse(yaml).expect("inheritedBy failed to parse");

        applies_to_dotnet(file.inherited_by.as_ref())
    }

    #[test]
    fn unscoped_files_always_apply() {
        assert!(applies_to_dotnet(None));
        assert!(applies_to_dotnet(Some(&scope(None, None))));
    }

    #[test]
    fn matching_toolchain_or_language_applies() {
        assert!(applies_to_dotnet(Some(&scope(Some(&["dotnet"]), None))));
        assert!(applies_to_dotnet(Some(&scope(Some(&["DotNet"]), None))));
        assert!(applies_to_dotnet(Some(&scope(None, Some(&["csharp"])))));
        assert!(applies_to_dotnet(Some(&scope(None, Some(&["F#"])))));
        assert!(applies_to_dotnet(Some(&scope(
            None,
            Some(&["visualbasic"])
        ))));
    }

    #[test]
    fn a_scope_naming_only_other_toolchains_does_not_apply() {
        assert!(!applies_to_dotnet(Some(&scope(Some(&["node"]), None))));
        assert!(!applies_to_dotnet(Some(&scope(
            None,
            Some(&["typescript"])
        ))));
        assert!(!applies_to_dotnet(Some(&scope(
            Some(&["rust"]),
            Some(&["go"])
        ))));
    }

    #[test]
    fn accepts_every_shape_moon_allows_for_a_condition() {
        // moon types these as `OneOrMany<..>` / a clause, so all of these are
        // legal files. Declaring only the list form made the scalar and clause
        // forms a hard parse error, which dropped the file's task ids from the
        // reserved set entirely — the one outcome this module exists to avoid.
        assert!(applies("inheritedBy:\n  toolchains: dotnet\n"));
        assert!(applies("inheritedBy:\n  toolchains: [dotnet]\n"));
        assert!(applies("inheritedBy:\n  toolchains:\n    or: dotnet\n"));
        assert!(applies(
            "inheritedBy:\n  toolchains:\n    or: [dotnet, node]\n"
        ));
        assert!(applies("inheritedBy:\n  toolchains:\n    not: [node]\n"));

        assert!(!applies("inheritedBy:\n  toolchains: node\n"));
        assert!(!applies("inheritedBy:\n  toolchains:\n    not: [dotnet]\n"));
        assert!(!applies("inheritedBy:\n  toolchains:\n    or: [node]\n"));
    }

    #[test]
    fn honours_moons_singular_field_aliases() {
        // moon aliases `toolchains` to `toolchain` and `languages` to
        // `language`; this repo's own `.moon/tasks/rust.yml` uses the singular.
        assert!(applies("inheritedBy:\n  toolchain: dotnet\n"));
        assert!(!applies("inheritedBy:\n  toolchain: node\n"));
        assert!(applies("inheritedBy:\n  language: csharp\n"));
        assert!(!applies("inheritedBy:\n  language: typescript\n"));
    }

    #[test]
    fn recognizes_the_dotnet_language_alias() {
        // `.net` is moon's own alias for `LanguageType::DotNet`.
        assert!(applies_to_dotnet(Some(&scope(None, Some(&[".net"])))));
    }

    #[test]
    fn conditions_moon_scopes_by_alone_never_exclude_dotnet() {
        // `layers`/`stacks`/`tags`/`files` cannot rule a dotnet project out, so
        // a file scoped only by them still has its ids reserved.
        assert!(applies("inheritedBy:\n  layers: [application]\n"));
        assert!(applies("inheritedBy:\n  stacks: [backend]\n"));
        assert!(applies("inheritedBy:\n  order: 50\n"));
    }

    #[test]
    fn a_match_in_either_dimension_is_enough() {
        // Scoped to another toolchain but a .NET language, or vice versa.
        assert!(applies_to_dotnet(Some(&scope(
            Some(&["node"]),
            Some(&["csharp"])
        ))));
        assert!(applies_to_dotnet(Some(&scope(
            Some(&["dotnet"]),
            Some(&["typescript"])
        ))));
    }
}
