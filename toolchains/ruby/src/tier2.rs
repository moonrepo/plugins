use crate::config::{DEFAULT_NON_PRODUCTION_GROUPS, RubyToolchainConfig};
use crate::gemfile;
use extism_pdk::*;
use moon_config::{DependencyScope, VersionSpec};
use moon_pdk::{
    VirtualPathExt, get_host_env_var, locate_root, locate_root_many, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use rubund::parser::{SourceType, parse_lockfile};
use ruby_tool::parse_ruby_version_file;
use starbase_utils::fs;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

// Files that mark a Bundler dependency root, lockfiles first (most precise),
// then manifests. Includes Bundler's alternative `gems.rb`/`gems.locked` names.
const ROOT_FILES: [&str; 4] = ["Gemfile.lock", "gems.locked", "Gemfile", "gems.rb"];

// Bundler prefers `gems.rb` when both exist
const MANIFEST_FILES: [&str; 2] = ["gems.rb", "Gemfile"];

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    // Bundler has no workspace concept (unlike npm/uv/Cargo), so each project
    // is its own dependency root and there are no members to report. Walk
    // upward for the nearest Gemfile/Gemfile.lock. If none is found, `root`
    // stays `None` and moon skips install steps for this project.
    Ok(Json(LocateDependenciesRootOutput {
        root: locate_root_many(&input.starting_dir, &ROOT_FILES),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config = parse_toolchain_config_schema::<RubyToolchainConfig>(input.toolchain_config)?;
    let mut output = SetupEnvironmentOutput::default();

    // Install gems into a project-local directory (the Ruby analog of a venv /
    // node_modules) so installs are isolated, cacheable, and don't pollute a
    // shared Ruby. Persisted via `bundle config` rather than the deprecated
    // `bundle install --path` flag, and only when it changed, as moon skips
    // the setup when the commands match the previous run.
    if get_local_bundle_setting(&input.root, "BUNDLE_PATH")?.as_deref()
        != Some(config.bundle_path.as_str())
    {
        output.commands.push(
            ExecCommandInput::new(
                "bundle",
                [
                    "config",
                    "set",
                    "--local",
                    "path",
                    config.bundle_path.as_str(),
                ],
            )
            .cwd(input.root.clone())
            .into(),
        );
    }

    Ok(Json(output))
}

/// Read a setting from the app's local Bundler config, which is flat YAML with one
/// `BUNDLE_<KEY>: "<value>"` per line. Returns `None` when `BUNDLE_APP_CONFIG` moves
/// the config outside the workspace, like `/usr/local/bundle` in the ruby image.
fn get_local_bundle_setting(root: &VirtualPath, key: &str) -> AnyResult<Option<String>> {
    let path = root.join(".bundle/config");

    if !path.exists() || get_host_env_var("BUNDLE_APP_CONFIG")?.is_some() {
        return Ok(None);
    }

    Ok(fs::read_file(&path)?.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;

        (name.trim() == key).then(|| value.trim().trim_matches('"').to_owned())
    }))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let config = parse_toolchain_config_schema::<RubyToolchainConfig>(input.toolchain_config)?;
    let mut output = InstallDependenciesOutput::default();

    let mut command = ExecCommandInput::new("bundle", ["install"]);
    command
        .args
        .extend(config.bundler_install_args.iter().cloned());
    command.cwd = Some(input.root.clone());

    // Not persisted with `bundle config`, so it doesn't block
    // `bundle add` and `bundle update` outside of moon
    if config.frozen {
        command.env.insert("BUNDLE_FROZEN".into(), "true".into());
    }

    // Production installs exclude dev/test groups. We use `BUNDLE_WITHOUT`
    // (colon-separated) rather than the `--without` flag, which is removed in
    // modern Bundler. Docker prune also persists it for `bundle exec`.
    if input.production {
        command.env.insert(
            "BUNDLE_WITHOUT".into(),
            config.get_non_production_groups().join(":"),
        );
    }

    // Bundler's resolver already produces a single flat gem set, so there is
    // no separate dedupe step (unlike npm/yarn).
    output.install_command = Some(command.into());

    Ok(Json(output))
}

/// Put the dependency root's binstubs directory (`bin/`, from Rails or
/// `bundle binstubs`) on `PATH`, so tasks can run `rails` or `rspec` directly.
fn gather_binstub_paths(
    context: &MoonContext,
    project: &ProjectFragment,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(root) = locate_root_many(&context.get_project_root(project), &ROOT_FILES)
        && root.join("bin").exists()
        && let Some(real) = root.to_real_path()?
    {
        paths.push(real.join("bin").to_path_buf());
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();

    gather_binstub_paths(&input.context, &input.project, &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    gather_binstub_paths(&input.context, &input.project, &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();
    let content = fs::read_file(&input.path)?;
    let lock = parse_lockfile(&content);

    if !content.trim().is_empty()
        && lock.sources.is_empty()
        && lock.specs.is_empty()
        && lock.dependencies.is_empty()
    {
        return Ok(Json(output));
    }

    for spec in lock.specs {
        let (version, platform) = split_platform(spec.version);
        let mut dep = LockDependency {
            version: VersionSpec::parse(version).ok(),
            ..Default::default()
        };

        // Stash provenance in `meta`: platform, then git revision, then path.
        if let Some(platform) = platform {
            dep.meta = Some(platform.to_string());
        } else if let Some(source) = lock.sources.get(spec.source_index) {
            match source.type_ {
                SourceType::Git => dep.meta = source.revision.map(str::to_owned),
                SourceType::Path => {
                    let path = source.path.unwrap_or(source.remote);

                    if !path.is_empty() {
                        dep.meta = Some(format!("path:{path}"));
                    }
                }
                SourceType::Gem => {}
            }
        }

        if let Some((_, _, sha)) = lock.checksums.iter().find(|(name, checksum_version, _)| {
            *name == spec.name && *checksum_version == spec.version
        }) {
            dep.hash = Some((*sha).to_string());
        }

        output
            .dependencies
            .entry(spec.name.to_string())
            .or_default()
            .push(dep);
    }

    Ok(Json(output))
}

/// Split a lockfile version token into (version, platform). Gem versions never
/// contain `-`; in a spec line a trailing `-<platform>` denotes a native gem.
fn split_platform(raw: &str) -> (&str, Option<&str>) {
    match raw.split_once('-') {
        Some((version, platform)) => (version, Some(platform)),
        None => (raw, None),
    }
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();

    if matches!(
        input.path.file_name().and_then(|name| name.to_str()),
        Some("Gemfile" | "gems.rb")
    ) {
        let content = fs::read_file(&input.path)?;

        for gem in gemfile::parse_path_gems(&content) {
            insert_path_manifest_dep(&mut output, gem);
        }
    }

    Ok(Json(output))
}

fn insert_path_manifest_dep(output: &mut ParseManifestOutput, gem: gemfile::PathGem) {
    let dep = ManifestDependency::path(PathBuf::from(gem.path));

    match groups_to_scope(&gem.groups, &DEFAULT_NON_PRODUCTION_GROUPS) {
        DependencyScope::Development => output.dev_dependencies.insert(gem.name, dep),
        _ => output.dependencies.insert(gem.name, dep),
    };
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config = parse_toolchain_config_schema::<RubyToolchainConfig>(input.toolchain_config)?;
    let non_production_groups = config.get_non_production_groups();
    let mut output = ExtendProjectGraphOutput::default();

    // Map each project's source directory to its id, so we can resolve a
    // `path:` gem reference back to the project it points at.
    let mut by_source: HashMap<PathBuf, Id> = HashMap::new();
    for (id, source) in &input.project_sources {
        by_source.insert(PathBuf::from(source), id.clone());
    }

    for (id, source) in &input.project_sources {
        let project_root = input.context.get_project_root_from_source(source);

        let Some(gemfile_path) = MANIFEST_FILES
            .iter()
            .map(|name| project_root.join(name))
            .find(|path| path.exists())
        else {
            continue;
        };

        let Ok(content) = fs::read_file(&gemfile_path) else {
            continue;
        };

        let mut project_output = ExtendProjectOutput::default();

        // Internal project relationships are expressed via `path:` gems, since
        // Bundler has no workspace concept.
        for gem in gemfile::parse_path_gems(&content) {
            // The join still needs normalizing to fold the `../..` in the
            // gem's relative path down to a workspace-relative source.
            let resolved = normalize(&Path::new(source).join(&gem.path));

            if let Some(dep_id) = by_source.get(&resolved)
                && dep_id != id
            {
                project_output.dependencies.push(ProjectDependency {
                    id: dep_id.clone(),
                    scope: groups_to_scope(&gem.groups, &non_production_groups),
                    via: Some(format!("path gem {}", gem.name)),
                });
            }
        }

        if !project_output.dependencies.is_empty() {
            output.extended_projects.insert(id.clone(), project_output);
        }

        output.input_files.push(gemfile_path);
    }

    Ok(Json(output))
}

fn groups_to_scope(groups: &[String], non_production_groups: &[&str]) -> DependencyScope {
    if groups
        .iter()
        .any(|group| non_production_groups.contains(&group.as_str()))
    {
        DependencyScope::Development
    } else {
        DependencyScope::Production
    }
}

/// Resolve `.`/`..` components in a relative path without touching the
/// filesystem, so `apps/web` + `../../libs/billing` -> `libs/billing`.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }

    out
}
