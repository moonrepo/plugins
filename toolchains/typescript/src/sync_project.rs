use crate::config::TypeScriptConfig;
use moon_common::{path::to_relative_virtual_string, Id};
use moon_pdk::{
    is_project_toolchain_enabled, map_miette_error, AnyResult, MoonContext, VirtualPath,
};
use moon_project::Project;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::json;
use typescript_tsconfig_json::{CompilerPath, ExtendsField, TsConfigJson};

fn create_missing_tsconfig(
    context: &MoonContext,
    config: &TypeScriptConfig,
    project: &Project,
) -> AnyResult<Option<VirtualPath>> {
    let project_root = context.workspace_root.join(project.source.as_str());
    let project_tsconfig_path = project_root.join(&config.project_config_file_name);

    if project_tsconfig_path.exists() {
        return Ok(None);
    }

    let options_tsconfig_path = context
        .workspace_root
        .join(&config.root)
        .join(&config.root_options_config_file_name);

    let json = TsConfigJson {
        extends: Some(ExtendsField::Single(
            to_relative_virtual_string(options_tsconfig_path.any_path(), project_root.any_path())
                .map_err(map_miette_error)?,
        )),
        include: Some(vec![CompilerPath::from("**/*")]),
        references: Some(vec![]),
        ..TsConfigJson::default()
    };

    json::write_file(&project_tsconfig_path, &json, true)?;

    Ok(Some(project_tsconfig_path))
}

pub fn sync_project_references(
    context: &MoonContext,
    config: TypeScriptConfig,
    project: Project,
    dependencies: FxHashMap<Id, Project>,
) -> AnyResult<Vec<VirtualPath>> {
    let mut tsconfig_project_refs = FxHashSet::default();
    let mut changed_files = vec![];

    for dep_config in &project.dependencies {
        let Some(dep_project) = dependencies.get(&dep_config.id) else {
            continue;
        };

        if dep_config.is_root_scope()
            || dep_config.is_build_scope()
            || dep_project.is_root_level()
            || !is_project_toolchain_enabled(dep_project, "typescript")
        {
            continue;
        }

        if context
            .workspace_root
            .join(dep_project.source.as_str())
            .join(&config.project_config_file_name)
            .exists()
        {
            tsconfig_project_refs.insert(dep_project.source.clone());
        }
    }

    if config.sync_project_references {
        // Auto-create a `tsconfig.json` if configured and applicable
        if config.create_missing_config {
            if let Some(file) = create_missing_tsconfig(&context, &config, &project)? {
                changed_files.push(file);
            }
        }

        // Sync project reference to the root `tsconfig.json`
        // if self.sync_as_root_project_reference()? {
        //     mutated = true;
        // }
    }

    Ok(changed_files)
}
