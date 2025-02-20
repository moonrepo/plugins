use crate::config::TypeScriptConfig;
use crate::tsconfig_json::TsConfigJson;
use moon_common::{path::is_root_level_source, Id};
use moon_config::DependencyScope;
use moon_pdk::{is_project_toolchain_enabled, AnyResult, MoonContext, VirtualPath};
use moon_project::ProjectFragment;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use typescript_tsconfig_json::{CompilerOptionsPathsMap, CompilerPath, ExtendsField};

pub struct ReferenceData {
    package_name: Option<String>,
    path: VirtualPath, // absolute
}

#[derive(Deserialize)]
struct PackageJson {
    name: Option<String>,
}

fn create_missing_tsconfig(
    context: &MoonContext,
    config: &TypeScriptConfig,
    project: &ProjectFragment,
) -> AnyResult<Option<VirtualPath>> {
    let project_root = context.workspace_root.join(&project.source);
    let project_tsconfig_path = project_root.join(&config.project_config_file_name);

    if project_tsconfig_path.exists() {
        return Ok(None);
    }

    let options_tsconfig_path = context
        .workspace_root
        .join(&config.root)
        .join(&config.root_options_config_file_name);

    let mut tsconfig = TsConfigJson::new(project_tsconfig_path);
    tsconfig.extends = Some(ExtendsField::Single(
        tsconfig.to_relative_path(options_tsconfig_path)?,
    ));
    tsconfig.include = Some(vec![CompilerPath::from("**/*")]);
    tsconfig.references = Some(vec![]);
    tsconfig.save()
}

fn sync_root_project_reference(
    context: &MoonContext,
    config: &TypeScriptConfig,
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

    let root_tsconfig_path = types_root.join(&config.root_config_file_name);

    // Don't create a file if it doesn't exist
    if !root_tsconfig_path.exists() {
        return Ok(None);
    }

    let mut tsconfig = TsConfigJson::load(root_tsconfig_path)?;
    tsconfig.add_project_ref(&project_root, &config.project_config_file_name)?;
    tsconfig.save()
}

pub fn sync_project_options(
    context: &MoonContext,
    config: &TypeScriptConfig,
    project: &ProjectFragment,
    project_refs: &FxHashMap<Id, ReferenceData>,
) -> AnyResult<Option<VirtualPath>> {
    let types_root = context.workspace_root.join(&config.root);
    let project_root = context.workspace_root.join(&project.source);
    let project_tsconfig_path = project_root.join(&config.project_config_file_name);

    if !project_tsconfig_path.exists() {
        return Ok(None);
    }

    let mut tsconfig = TsConfigJson::load(project_tsconfig_path)?;

    // Add shared types to `include`
    if config.include_shared_types && types_root.join("types").exists() {
        tsconfig.add_include(&types_root.join("types/**/*"))?;
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
            if config.sync_project_references_to_paths {
                if let Some(package_name) = &project_ref.package_name {
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
    context: &MoonContext,
    config: &TypeScriptConfig,
    project: &ProjectFragment,
    dependencies: &[ProjectFragment],
) -> AnyResult<Vec<VirtualPath>> {
    let mut project_refs = FxHashMap::default();
    let mut changed_files = vec![];

    for dep_project in dependencies {
        if !is_project_toolchain_enabled(dep_project, "typescript")
            || is_root_level_source(&dep_project.source)
            || dep_project.dependency_scope.is_some_and(|scope| {
                matches!(scope, DependencyScope::Build | DependencyScope::Root)
            })
        {
            continue;
        }

        let dep_project_root = context.workspace_root.join(&dep_project.source);
        let tsconfig_path = dep_project_root.join(&config.project_config_file_name);
        let package_path = dep_project_root.join("package.json");

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
        if config.create_missing_config {
            if let Some(file) = create_missing_tsconfig(context, config, project)? {
                changed_files.push(file);
            }
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
