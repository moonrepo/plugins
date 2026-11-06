use crate::config::{DEFAULT_NON_PRODUCTION_GROUPS, RubyToolchainConfig};
use crate::gemfile;
use extism_pdk::*;
use moon_config::{DependencyScope, UnresolvedVersionSpec, VersionSpec};
use moon_pdk::{
    HostLogInput, VirtualPathExt, get_host_env_var, get_host_environment, get_toolchain_config,
    host_log, locate_root_many, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use rubund::parser::{SourceType, parse_lockfile};
use starbase_utils::fs;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

// Files that mark a Bundler dependency root, lockfiles first (most precise),
// then manifests. Includes Bundler's alternative `gems.rb`/`gems.locked` names.
const ROOT_FILES: [&str; 4] = ["Gemfile.lock", "gems.locked", "Gemfile", "gems.rb"];

const LOCK_FILES: [&str; 2] = ["Gemfile.lock", "gems.locked"];

// Bundler prefers `gems.rb` when both exist
const MANIFEST_FILES: [&str; 2] = ["gems.rb", "Gemfile"];

fn find_manifest(dir: &VirtualPath) -> Option<VirtualPath> {
    MANIFEST_FILES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut output = LocateDependenciesRootOutput::default();

    // Walk upward for the nearest Gemfile/Gemfile.lock. If none is found, `root`
    // stays `None` and moon skips install steps for this project.
    if let Some(root) = locate_root_many(&input.starting_dir, &ROOT_FILES) {
        // Bundler has no workspaces, but local gems in subdirectories, like
        // `libs/billing` in a shared Gemfile, use the root's bundle. Report them
        // as members, otherwise moon never installs it for their projects.
        let members = match find_manifest(&root) {
            Some(manifest) => gemfile::parse_gems(&fs::read_file(manifest)?)
                .into_iter()
                .filter_map(|gem| get_member_dir(gem.path.as_deref()?))
                .collect::<Vec<_>>(),
            None => vec![],
        };

        output.members = (!members.is_empty()).then_some(members);
        output.root = Some(root);
    }

    Ok(Json(output))
}

/// A local gem's directory, when it's within the dependency root.
fn get_member_dir(path: &str) -> Option<String> {
    let path = Path::new(path);

    if path.is_absolute() || path.components().any(|comp| comp == Component::ParentDir) {
        return None;
    }

    let dir = normalize(path).to_string_lossy().into_owned();

    (!dir.is_empty()).then_some(dir)
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config = parse_toolchain_config_schema::<RubyToolchainConfig>(input.toolchain_config)?;
    let bundle_path = get_bundle_path()?;
    let mut output = SetupEnvironmentOutput::default();

    if config.bundle_path != bundle_path {
        host_log!(
            warn,
            "Ignoring a project's <property>bundlePath</property>, as moon checks for installed gems in the workspace's"
        );
    }

    // Install gems into a project-local directory (the Ruby analog of a venv /
    // node_modules) so installs are isolated, cacheable, and don't pollute a
    // shared Ruby. Persisted via `bundle config` rather than the deprecated
    // `bundle install --path` flag, and only when it changed, as moon skips
    // the setup when the commands match the previous run.
    if get_local_bundle_setting(&input.root, "BUNDLE_PATH")?.as_deref()
        != Some(bundle_path.as_str())
    {
        let mut command = ExecCommandInput::new(
            "bundle",
            ["config", "set", "--local", "path", bundle_path.as_str()],
        )
        .cwd(input.root.clone());

        config.pin_version(&mut command);
        output.commands.push(command.into());
    }

    Ok(Json(output))
}

/// The workspace's `bundlePath`. moon checks for installed gems in it, as the
/// toolchain's vendor directory, so a project's would reinstall them every run.
pub(crate) fn get_bundle_path() -> AnyResult<String> {
    Ok(get_toolchain_config::<RubyToolchainConfig>()?.bundle_path)
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

    // Unlike npm or pnpm, Bundler doesn't freeze the lockfile in CI,
    // so an out-of-date lockfile would be silently rewritten
    let frozen = config.frozen
        || (get_host_environment()?.ci
            && LOCK_FILES.iter().any(|name| input.root.join(name).exists()));

    // Not persisted with `bundle config`, so it doesn't block
    // `bundle add` and `bundle update` outside of moon
    if frozen {
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

    config.pin_version(&mut command);

    // Bundler's resolver already produces a single flat gem set, so there is
    // no separate dedupe step (unlike npm/yarn).
    output.install_command = Some(command.into());

    Ok(Json(output))
}

/// Put the dependency root's binstubs directory (`bin/`, from Rails or
/// `bundle binstubs`) on `PATH`, so tasks can run `rails` or `rspec` directly.
fn gather_binstub_paths(root: &VirtualPath, paths: &mut Vec<PathBuf>) -> AnyResult<()> {
    if root.join("bin").exists()
        && let Some(real) = root.to_real_path()?
    {
        paths.push(real.join("bin").to_path_buf());
    }

    Ok(())
}

/// Whether the command is a gem executable that's only in the bundle,
/// at `<bundlePath>/ruby/<abi>/bin`, and not a binstub.
fn is_bundled_executable(root: &VirtualPath, command: &str) -> AnyResult<bool> {
    if matches!(command, "ruby" | "bundle" | "bundler" | "gem")
        || command.contains(['/', '\\'])
        || root.join("bin").join(command).exists()
    {
        return Ok(false);
    }

    let abi_dirs = root.join(get_bundle_path()?).join("ruby");

    if !abi_dirs.exists() {
        return Ok(false);
    }

    Ok(fs::read_dir(&abi_dirs)?
        .iter()
        .any(|abi| abi.path().join("bin").join(command).exists()))
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();

    let Some(root) = locate_root_many(&input.context.get_project_root(&input.project), &ROOT_FILES)
    else {
        return Ok(Json(output));
    };

    gather_binstub_paths(&root, &mut output.paths)?;

    // Gem executables like `rspec` aren't on `PATH`, so run them with Bundler
    if is_bundled_executable(&root, &input.command)? {
        output.command = Some("bundle".into());
        output.args = Some(Extend::Prepend(vec!["exec".into(), input.command]));
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();

    if let Some(root) =
        locate_root_many(&input.context.get_project_root(&input.project), &ROOT_FILES)
    {
        gather_binstub_paths(&root, &mut output.paths)?;
    }

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
            version: VersionSpec::parse(normalize_gem_version(version)).ok(),
            ..Default::default()
        };

        // Stash provenance in `meta`: platform, then git revision, then path.
        if let Some(platform) = platform {
            dep.meta = Some(platform.to_string());
        } else if let Some(source) = lock.sources.get(spec.source_index) {
            match source.type_ {
                // moon hashes this over the version, which a git gem
                // often keeps between revisions
                SourceType::Git => dep.hash = source.revision.map(str::to_owned),
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

/// RubyGems versions have any number of segments, and a letter starts a
/// prerelease, so `7.1.3.4`, `3.1` and `1.0.0.pre` aren't semver. moon only uses
/// these to hash tasks, against the `*` requirements from `parse_manifest`, which
/// don't match prereleases. So pad to 3 numeric segments, and keep the rest as
/// build metadata, like `7.1.3+4` and `1.0.0+pre`, which is still hashed.
fn normalize_gem_version(version: &str) -> String {
    let mut segments = version.split('.').peekable();
    let mut release = vec![];

    while release.len() < 3
        && let Some(number) = segments
            .peek()
            .and_then(|segment| segment.parse::<u64>().ok())
    {
        release.push(number.to_string());
        segments.next();
    }

    release.resize(3, "0".into());

    let mut normalized = release.join(".");
    let build = segments.collect::<Vec<_>>().join(".");

    if !build.is_empty() {
        normalized.push('+');
        normalized.push_str(&build);
    }

    normalized
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

        for gem in gemfile::parse_gems(&content) {
            insert_manifest_dep(&mut output, gem)?;
        }
    }

    Ok(Json(output))
}

fn insert_manifest_dep(output: &mut ParseManifestOutput, gem: gemfile::Gem) -> AnyResult<()> {
    let dep = match gem.path {
        Some(path) => ManifestDependency::path(PathBuf::from(path)),
        // Requirements like `~> 7.1` aren't translated, as the hasher
        // records the locked version that `*` matches instead
        None => ManifestDependency::new(UnresolvedVersionSpec::parse("*")?),
    };

    match groups_to_scope(&gem.groups, &DEFAULT_NON_PRODUCTION_GROUPS) {
        DependencyScope::Development => output.dev_dependencies.insert(gem.name, dep),
        _ => output.dependencies.insert(gem.name, dep),
    };

    Ok(())
}

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let config = parse_toolchain_config_schema::<RubyToolchainConfig>(input.toolchain_config)?;
    let non_production_groups = config.get_non_production_groups();
    let mut output = ExtendProjectGraphOutput::default();

    // Map each project's source directory to its id, so we can resolve a
    // `path:` gem reference back to the project it points at, and each of its
    // gem names, for gemspec dependencies and gems that Bundler finds by name.
    let mut by_source: HashMap<PathBuf, &Id> = HashMap::new();
    let mut by_name: HashMap<&str, &Id> = HashMap::new();
    let mut gemspecs: HashMap<&Id, Vec<ProjectGemspec>> = HashMap::new();

    for (id, source) in &input.project_sources {
        by_source.insert(PathBuf::from(source), id);
        gemspecs.insert(
            id,
            read_gemspecs(&input.context.get_project_root_from_source(source)),
        );
    }

    for id in input.project_sources.keys() {
        for spec in &gemspecs[id] {
            by_name.insert(&spec.name, id);
        }
    }

    for (id, source) in &input.project_sources {
        let project_root = input.context.get_project_root_from_source(source);
        let mut project_output = ExtendProjectOutput::default();

        // Internal project relationships are expressed via `path:` gems, since
        // Bundler has no workspace concept.
        if let Some(manifest) = find_manifest(&project_root)
            && let Ok(content) = fs::read_file(&manifest)
        {
            for gem in gemfile::parse_gems(&content) {
                let Some(path) = &gem.path else {
                    continue;
                };

                // The join still needs normalizing to fold the `../..` in the
                // gem's relative path down to a workspace-relative source.
                let resolved = normalize(&Path::new(source).join(path));

                if let Some(dep_id) = by_source
                    .get(&resolved)
                    .or_else(|| by_name.get(gem.name.as_str()))
                {
                    add_project_dependency(
                        &mut project_output,
                        id,
                        dep_id,
                        groups_to_scope(&gem.groups, &non_production_groups),
                        format!("path gem {}", gem.name),
                    );
                }
            }

            output.input_files.push(manifest);
        }

        // Libraries that share a Gemfile depend on each other in their gemspecs
        let specs = &gemspecs[id];

        for spec in specs {
            for dep in &spec.dependencies {
                if let Some(dep_id) = by_name.get(dep.name.as_str()) {
                    add_project_dependency(
                        &mut project_output,
                        id,
                        dep_id,
                        if dep.development {
                            DependencyScope::Development
                        } else {
                            DependencyScope::Production
                        },
                        format!("gemspec dependency {}", dep.name),
                    );
                }
            }

            output.input_files.push(spec.path.clone());
        }

        if let [spec] = specs.as_slice() {
            project_output.alias = Some(spec.name.clone());
        }

        if !project_output.dependencies.is_empty() || project_output.alias.is_some() {
            output.extended_projects.insert(id.clone(), project_output);
        }
    }

    Ok(Json(output))
}

struct ProjectGemspec {
    path: VirtualPath,
    name: String,
    dependencies: Vec<gemfile::GemspecDependency>,
}

/// Read the gemspecs in a project's root. The gem name defaults to the file stem.
fn read_gemspecs(dir: &VirtualPath) -> Vec<ProjectGemspec> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };

    let mut gemspecs = entries
        .into_iter()
        .filter_map(|entry| {
            let file_name = entry.file_name().into_string().ok()?;
            let stem = file_name.strip_suffix(".gemspec")?;
            let path = dir.join(&file_name);
            let gemspec = gemfile::parse_gemspec(&fs::read_file(&path).ok()?);

            Some(ProjectGemspec {
                name: gemspec.name.unwrap_or_else(|| stem.to_owned()),
                dependencies: gemspec.dependencies,
                path,
            })
        })
        .collect::<Vec<_>>();

    // Directory order isn't stable
    gemspecs.sort_by(|a, b| a.name.cmp(&b.name));
    gemspecs
}

/// Skips self references, and gems that are in both the Gemfile and gemspec.
fn add_project_dependency(
    output: &mut ExtendProjectOutput,
    id: &Id,
    dep_id: &Id,
    scope: DependencyScope,
    via: String,
) {
    if dep_id != id && !output.dependencies.iter().any(|dep| &dep.id == dep_id) {
        output.dependencies.push(ProjectDependency {
            id: dep_id.clone(),
            scope,
            via: Some(via),
        });
    }
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
