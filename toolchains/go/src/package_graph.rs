use crate::go_mod::{GoMod, parse_go_mod};
use moon_pdk::exec;
use moon_pdk_api::*;
use starbase_utils::fs;
use std::collections::BTreeMap;

// The `go list` half of the package graph: every package a directory's
// packages depend on, as canonical import paths.
pub fn execute_go_list(
    dir: &VirtualPath,
    packages: &[String],
    test: bool,
) -> AnyResult<Vec<String>> {
    let mut args = vec![
        "list",
        "-deps",
        "-f",
        "{{if .Module}}{{.ImportPath}}{{end}}",
    ];

    if test {
        args.push("-test");
    }

    // Scan all packages recursively by default so that dependencies imported
    // only from subdirectories (internal/, pkg/, ...) are also inferred.
    if packages.is_empty() {
        args.push("./...");
    } else {
        for package in packages {
            args.push(package.as_str());
        }
    }

    let result = exec(ExecCommandInput::pipe("go", args).cwd(dir.to_owned()))?;

    if result.exit_code != 0 {
        return Ok(vec![]);
    }

    Ok(result
        .stdout
        .lines()
        .filter_map(|line| {
            // Test binary pseudo-packages render as `pkg [pkg.test]`;
            // only the real package path participates in matching.
            let import_path = line.trim().split(' ').next().unwrap_or_default();

            (!import_path.is_empty()).then(|| import_path.to_owned())
        })
        .collect())
}

// Whether `import_path` is the package at `prefix` itself, or nested beneath
// it.
pub fn import_within(import_path: &str, prefix: &str) -> bool {
    import_path == prefix
        || import_path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

// Workspace-relative dirs that may own a directory's `go.mod`: the dir
// itself, then each ancestor up to the workspace root ("").
fn module_dir_candidates(source: &str) -> Vec<&str> {
    let mut candidates = vec![];
    let mut dir = source;

    while !dir.is_empty() {
        candidates.push(dir);
        dir = dir.rfind('/').map_or("", |index| &dir[..index]);
    }

    candidates.push("");
    candidates
}

// Resolves workspace directories to the import paths their packages live
// under, caching each `go.mod` it parses along the way. A `None` cache entry
// memoizes "no usable `go.mod` here", so ancestors shared between
// directories only hit the disk once.
pub struct ModuleResolver {
    workspace_root: VirtualPath,
    go_mods: BTreeMap<String, Option<GoMod>>,
}

impl ModuleResolver {
    pub fn new(workspace_root: VirtualPath) -> Self {
        Self {
            workspace_root,
            go_mods: BTreeMap::default(),
        }
    }

    // The canonical import path of a workspace-relative dir: the module path
    // of the nearest `go.mod` at or above it (bounded by the workspace
    // root), joined with the dir's path relative to that module root.
    // Returns the owning dir alongside so callers can key back into the
    // cache.
    pub fn import_path(&mut self, source: &str) -> AnyResult<Option<(String, String)>> {
        for dir in module_dir_candidates(source) {
            if !self.go_mods.contains_key(dir) {
                let go_mod_path = self.go_mod_path(dir);

                let manifest = if go_mod_path.exists() {
                    Some(parse_go_mod(fs::read_file(&go_mod_path)?)?)
                        .filter(|manifest| !manifest.module.is_empty())
                        .map(|mut manifest| {
                            // A module in a major version folder (v2+) is
                            // imported with the version suffix, even when the
                            // `module` directive omits it
                            // https://go.dev/ref/mod#major-version-suffixes
                            if let Some(version) = dir
                                .rsplit('/')
                                .next()
                                .filter(|segment| is_version_segment(segment))
                                && !is_version_segment(
                                    manifest.module.rsplit('/').next().unwrap_or_default(),
                                )
                            {
                                manifest.module = format!("{}/{version}", manifest.module);
                            }

                            manifest
                        })
                } else {
                    None
                };

                self.go_mods.insert(dir.to_owned(), manifest);
            }

            if let Some(manifest) = self.go_mods.get(dir).and_then(|entry| entry.as_ref()) {
                let relative = source[dir.len()..].trim_start_matches('/');

                let import_path = if relative.is_empty() {
                    manifest.module.clone()
                } else {
                    format!("{}/{relative}", manifest.module)
                };

                return Ok(Some((dir.to_owned(), import_path)));
            }
        }

        Ok(None)
    }

    // The parsed manifest owned by a dir previously resolved through
    // `import_path`, with any major version suffix already applied to its
    // module path.
    pub fn manifest(&self, dir: &str) -> Option<&GoMod> {
        self.go_mods.get(dir).and_then(|entry| entry.as_ref())
    }

    pub fn go_mod_path(&self, dir: &str) -> VirtualPath {
        if dir.is_empty() {
            self.workspace_root.join("go.mod")
        } else {
            self.workspace_root.join(dir).join("go.mod")
        }
    }
}

// Lexically resolve a relative path (`./`, `../`) against a workspace
// relative source, returning `None` when it escapes the workspace
pub fn resolve_source_path(base: &str, path: &str) -> Option<String> {
    if path.starts_with('/') {
        return None;
    }

    let mut segments = base
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .collect::<Vec<_>>();

    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            _ => segments.push(segment),
        }
    }

    Some(segments.join("/"))
}

// A major version suffix segment: v2, v3, etc, but never v0 or v1
// https://go.dev/ref/mod#major-version-suffixes
pub fn is_version_segment(segment: &str) -> bool {
    match segment.strip_prefix('v') {
        Some(digits) => {
            !digits.is_empty()
                && !digits.starts_with('0')
                && digits != "1"
                && digits.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}
