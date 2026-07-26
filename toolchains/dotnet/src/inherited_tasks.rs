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

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct InheritedByScope {
    toolchains: Option<Vec<String>>,
    languages: Option<Vec<String>>,
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

        if toolchains
            .iter()
            .any(|id| id.eq_ignore_ascii_case("dotnet"))
        {
            return true;
        }
    }

    if let Some(languages) = &scope.languages {
        scoped = true;

        if languages.iter().any(|lang| {
            matches!(
                lang.to_lowercase().as_str(),
                "csharp" | "c#" | "fsharp" | "f#" | "vb" | "visualbasic" | "dotnet"
            )
        }) {
            return true;
        }
    }

    !scoped
}

fn collect_yaml_files(dir: &VirtualPath, out: &mut Vec<VirtualPath>) {
    if let Ok(entries) = fs::read_dir(dir.any_path()) {
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

        // An unparseable file is moon's problem to report; there is nothing
        // for inference to yield to.
        if let Ok(parsed) = yaml::read_file::<InheritedTasksFile>(file.any_path())
            && applies_to_dotnet(parsed.inherited_by.as_ref())
        {
            let label = file.to_string();

            for id in parsed.tasks.into_keys() {
                ids.entry(id).or_insert_with(|| label.clone());
            }
        }
    }

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(toolchains: Option<&[&str]>, languages: Option<&[&str]>) -> InheritedByScope {
        InheritedByScope {
            toolchains: toolchains.map(|list| list.iter().map(|id| id.to_string()).collect()),
            languages: languages.map(|list| list.iter().map(|id| id.to_string()).collect()),
        }
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
