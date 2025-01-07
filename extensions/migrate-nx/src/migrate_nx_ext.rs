use crate::nx_migrator::NxMigrator;
use extism_pdk::*;
use moon_pdk::*;
use starbase_utils::{fs, glob, json};

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

#[plugin_fn]
pub fn register_extension(
    Json(_): Json<ExtensionMetadataInput>,
) -> FnResult<Json<ExtensionMetadataOutput>> {
    Ok(Json(ExtensionMetadataOutput {
        name: "Migrate Nx".into(),
        description: Some("Migrate an Nx repository to moon by converting all <file>nx.json</file> and <file>project.json</file> files into moon configuration files.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_schema: None,
    }))
}

#[derive(Args)]
pub struct MigrateNxExtensionArgs {
    #[arg(long)]
    pub bun: bool,
}

#[plugin_fn]
pub fn execute_extension(Json(input): Json<ExecuteExtensionInput>) -> FnResult<()> {
    let args = parse_args::<MigrateNxExtensionArgs>(&input.args)?;
    let workspace_root = &input.context.workspace_root;
    let mut migrator = NxMigrator::new(&input.context, args.bun)?;

    // Migrate the workspace config first, so we can handle projects
    let workspace_config_path = workspace_root.join("workspace.json");

    if workspace_config_path.exists() {
        host_log!(
            stdout,
            "Migrating workspace config <file>workspace.json</file>",
        );

        migrator.migrate_workspace_config(json::read_file(&workspace_config_path)?)?;

        fs::remove(workspace_config_path)?;
    }

    // Then the root nx config second, to handle project defaults
    let root_config_path = workspace_root.join("nx.json");

    if root_config_path.exists() {
        host_log!(stdout, "Migrating root config <file>nx.json</file>",);

        migrator.migrate_root_config(json::read_file(&root_config_path)?)?;

        fs::remove(root_config_path)?;
    }

    // And lastly, all project configs (and package.json to)
    for project_config_path in glob::walk_files(
        workspace_root,
        [
            "**/*/package.json",
            "**/*/project.json",
            "!**/node_modules/**/*",
            // The globstar above won't find the root files for some reason...
            "package.json",
            "project.json",
        ],
    )? {
        let rel_config_path = project_config_path.strip_prefix(workspace_root).unwrap();
        let project_source = rel_config_path.parent().unwrap().to_string_lossy();

        host_log!(
            stdout,
            "Migrating project config <file>{}</file>",
            rel_config_path.display()
        );

        if project_config_path
            .file_name()
            .is_some_and(|name| name == "package.json")
        {
            migrator.migrate_project_package_config(
                &project_source,
                json::read_file(&project_config_path)?,
            )?;

            // Don't delete package.json
        } else {
            migrator
                .migrate_project_config(&project_source, json::read_file(&project_config_path)?)?;

            fs::remove(project_config_path)?;
        }
    }

    // Fill in any missing but required settings
    migrator.use_default_settings()?;

    // Write the new config files
    migrator.inner.save_configs()?;

    host_log!(stdout, "Successfully migrated from Nx to moon!");

    Ok(())
}
