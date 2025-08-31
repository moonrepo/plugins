use crate::config::TypeScriptToolchainConfig;
use crate::context::TypeScriptContext;
use crate::tsconfig_json::TsConfigJson;
use moon_common::{Id, path::is_root_level_source};
use moon_config::DependencyScope;
use moon_pdk::is_project_toolchain_enabled;
use moon_pdk_api::{AnyResult, VirtualPath};
use moon_project::ProjectFragment;
use serde::Deserialize;
use std::collections::BTreeMap;
use typescript_tsconfig_json::{CompilerOptionsPathsMap, CompilerPath, ExtendsField};

#[derive(Debug)]
pub struct ReferenceData {
    package_name: Option<String>,
    path: VirtualPath, // absolute
}

#[derive(Deserialize)]
struct PackageJson {
    name: Option<String>,
}

fn create_missing_tsconfig(context: &TypeScriptContext) -> AnyResult<Option<VirtualPath>> {
    if context.project_config.exists() {
        return Ok(None);
    }

    let mut tsconfig = TsConfigJson::new(context.project_config.clone());
    tsconfig.extends = Some(ExtendsField::Single(
        tsconfig
            .to_relative_path(&context.root_options_config)?
            .into(),
    ));
    tsconfig.include = Some(vec![CompilerPath::from("**/*")]);
    tsconfig.references = Some(vec![]);
    let file = tsconfig.save_model()?;

    Ok(Some(file))
}

fn sync_root_project_reference(
    context: &TypeScriptContext,
    config: &TypeScriptToolchainConfig,
    project: &ProjectFragment,
) -> AnyResult<Option<VirtualPath>> {
    let project_root = context.workspace_root.join(&project.source);
    let types_root = context.workspace_root.join(&config.root);

    // Don't sync a root project to itself
    if (project_root == types_root || project_root == context.workspace_root)
        && config.project_config_file_name == config.root_config_file_name
    {
        return Ok(None);
    }

    // Don't create a file if it doesn't exist
    if !context.root_config.exists() {
        return Ok(None);
    }

    let mut tsconfig = TsConfigJson::load(context.root_config.clone())?;
    tsconfig.add_project_ref(&project_root, &config.project_config_file_name)?;
    tsconfig.save()
}

pub fn sync_project_options(
    context: &TypeScriptContext,
    config: &TypeScriptToolchainConfig,
    project: &ProjectFragment,
    project_refs: &BTreeMap<Id, ReferenceData>,
) -> AnyResult<Option<VirtualPath>> {
    let types_root = context.workspace_root.join(&config.root);

    if !context.project_config.exists() {
        return Ok(None);
    }

    let mut tsconfig = TsConfigJson::load(context.project_config.clone())?;

    // Add shared types to `include`
    let shared_types_root = types_root.join("types");

    if config.include_shared_types && shared_types_root.exists() {
        tsconfig.add_include(&shared_types_root.join("**/*"))?;

        // And also include as a project reference
        if config.sync_project_references
            && shared_types_root
                .join(&config.project_config_file_name)
                .exists()
        {
            tsconfig.add_project_ref(&shared_types_root, &config.project_config_file_name)?;
        }
    }

    // Sync project dependencies as project `references`
    if config.sync_project_references && !project_refs.is_empty() {
        for project_ref in project_refs.values() {
            tsconfig.add_project_ref(&project_ref.path, &config.project_config_file_name)?;

            // Include their sources
            if config.include_project_reference_sources {
                tsconfig.add_include(&project_ref.path.join("**/*"))?;
            }

            // Add compiler option paths
            if config.sync_project_references_to_paths
                && let Some(package_name) = &project_ref.package_name
            {
                let mut compiler_paths = CompilerOptionsPathsMap::default();
                let has_src_dir = project_ref.path.join("src").exists();

                for index in if has_src_dir {
                    vec!["src/index.ts", "src/index.tsx", "index.ts", "index.tsx"]
                } else {
                    vec!["index.ts", "index.tsx"]
                } {
                    let index_path = project_ref.path.join(index);

                    if index_path.exists() {
                        compiler_paths.insert(
                            package_name.to_owned(),
                            vec![CompilerPath::from(tsconfig.to_relative_path(index_path)?)],
                        );

                        break;
                    }
                }

                compiler_paths.insert(
                    format!("{package_name}/*"),
                    vec![CompilerPath::from(
                        tsconfig.to_relative_path(project_ref.path.join(if has_src_dir {
                            "src/*"
                        } else {
                            "*"
                        }))?,
                    )],
                );

                tsconfig.update_compiler_option_paths(compiler_paths);
            }
        }
    }

    // Route `outDir` to moon's cache
    if config.route_out_dir_to_cache {
        let out_dir = tsconfig.to_relative_path(
            context
                .workspace_root
                .join(".moon/cache/types")
                .join(&project.source),
        )?;

        tsconfig.update_compiler_options(|options| {
            if options.out_dir.is_none()
                || options
                    .out_dir
                    .as_ref()
                    .is_some_and(|dir| dir.as_str() != out_dir)
            {
                options.out_dir = Some(CompilerPath::from(out_dir));

                return true;
            }

            false
        });
    }

    tsconfig.save()
}

pub fn sync_project_references(
    context: &TypeScriptContext,
    config: &TypeScriptToolchainConfig,
    project: &ProjectFragment,
    dependencies: &[ProjectFragment],
) -> AnyResult<Vec<VirtualPath>> {
    let mut project_refs = BTreeMap::default();
    let mut changed_files = vec![];

    for dep_project in dependencies {
        if !is_project_toolchain_enabled(dep_project)
            || is_root_level_source(&dep_project.source)
            || dep_project.dependency_scope.is_some_and(|scope| {
                matches!(scope, DependencyScope::Build | DependencyScope::Root)
            })
        {
            continue;
        }

        let dep_project_root = context.workspace_root.join(&dep_project.source);
        let package_path = dep_project_root.join("package.json");
        let tsconfig_path = CompilerPath::resolve(
            dep_project_root
                .join(&config.project_config_file_name)
                .to_path_buf(),
        );

        if tsconfig_path.exists() {
            let mut data = ReferenceData {
                package_name: None,
                path: dep_project_root,
            };

            if config.sync_project_references_to_paths && package_path.exists() {
                let package: PackageJson = starbase_utils::json::read_file(&package_path)?;
                data.package_name = package.name;
            }

            project_refs.insert(dep_project.id.clone(), data);
        }
    }

    if config.sync_project_references {
        // Auto-create a `tsconfig.json` if configured and applicable
        if config.create_missing_config
            && let Some(file) = create_missing_tsconfig(context)?
        {
            changed_files.push(file);
        }

        // Sync project reference to the root `tsconfig.json`
        if let Some(file) = sync_root_project_reference(context, config, project)? {
            changed_files.push(file);
        }
    }

    // Sync compiler options to the project's `tsconfig.json`
    if let Some(file) = sync_project_options(context, config, project, &project_refs)? {
        changed_files.push(file);
    }

    Ok(changed_files)
}
