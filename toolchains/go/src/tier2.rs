use std::str::FromStr;

use crate::go_sum::GoSum;
use crate::go_work::GoWork;
use extism_pdk::*;
use gomod_parser::GoMod;
use moon_pdk_api::*;
use starbase_utils::fs;

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let mut output = ExtendProjectGraphOutput::default();

    // Extract the module name as an alias
    for (id, source) in input.project_sources {
        let go_mod_path = input.context.workspace_root.join(source).join("go.mod");

        if go_mod_path.exists() {
            let go_mod = fs::read_file(go_mod_path)?;
            let mut project_output = ExtendProjectOutput::default();
            let mut inject = false;

            for line in go_mod.lines() {
                if let Some(module) = line.strip_prefix("module") {
                    project_output.alias = Some(module.trim().to_owned());
                    inject = true;

                    break;
                }
            }

            if inject {
                output.extended_projects.insert(id, project_output);
            }
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn locate_dependencies_root(
    Json(input): Json<LocateDependenciesRootInput>,
) -> FnResult<Json<LocateDependenciesRootOutput>> {
    let mut output = LocateDependenciesRootOutput::default();

    let traverse = |starting_dir: &VirtualPath, file: &str| -> Option<VirtualPath> {
        let mut current_dir = Some(starting_dir.to_owned());

        while let Some(dir) = current_dir {
            if dir.join(file).exists() {
                return Some(dir);
            }

            current_dir = dir.parent();
        }

        None
    };

    // Find `go.work` first
    if let Some(root) = traverse(&input.starting_dir, "go.work") {
        let go_work = GoWork::parse(fs::read_file(root.join("go.work"))?)?;

        if !go_work.modules.is_empty() {
            output.members = Some(go_work.modules);
        }

        output.root = root.virtual_path();
    }

    // Then `go.sum` second
    if output.root.is_none() {
        if let Some(root) = traverse(&input.starting_dir, "go.sum") {
            output.root = root.virtual_path();
        }
    }

    // Otherwise assume `go.mod`
    if output.root.is_none() {
        output.root = input.starting_dir.virtual_path();
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn install_dependencies(
    Json(input): Json<InstallDependenciesInput>,
) -> FnResult<Json<InstallDependenciesOutput>> {
    let mut output = InstallDependenciesOutput::default();

    if input.root.join("go.work").exists() {
        output.install_command = Some(
            ExecCommandInput::new("go", ["work", "sync"])
                .cwd(input.root.clone())
                .into(),
        );
    }

    if input.root.join("go.mod").exists() {
        output.install_command = Some(
            ExecCommandInput::new("go", ["mod", "download"])
                .cwd(input.root.clone())
                .into(),
        );

        output.dedupe_command = Some(
            ExecCommandInput::new("go", ["mod", "tidy"])
                .cwd(input.root)
                .into(),
        );
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_lock(Json(input): Json<ParseLockInput>) -> FnResult<Json<ParseLockOutput>> {
    let mut output = ParseLockOutput::default();
    let go_sum = GoSum::parse(fs::read_file(input.path)?)?;

    for (module, entry) in go_sum.dependencies {
        output
            .dependencies
            .entry(module)
            .or_default()
            .push(LockDependency {
                hash: entry
                    .checksum
                    .strip_prefix("h1:") // sha256
                    .map(|hash| hash.to_owned()),
                version: Some(entry.version),
                ..Default::default()
            });
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();
    let go_mod =
        GoMod::from_str(&fs::read_file(input.path)?).map_err(|error| anyhow!("{error}"))?;

    for dep in go_mod.require {
        // Ignore transitive deps, as we only care about
        // direct project deps
        if dep.indirect {
            continue;
        }

        output.dependencies.insert(
            dep.module.module_path,
            ManifestDependency::Version(UnresolvedVersionSpec::parse(dep.module.version)?),
        );
    }

    Ok(Json(output))
}
