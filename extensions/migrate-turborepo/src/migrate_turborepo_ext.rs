use crate::turbo_migrator::TurboMigrator;
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
        name: "Migrate Turborepo".into(),
        description: Some("Migrate a Turborepo repository to moon by converting all <file>turbo.json</file> files into moon configuration files.".into()),
        plugin_version: env!("CARGO_PKG_VERSION").into(),
        config_schema: None,
    }))
}

#[derive(Args)]
pub struct MigrateTurborepoExtensionArgs {
    #[arg(long)]
    pub bun: bool,
}

#[plugin_fn]
pub fn execute_extension(Json(input): Json<ExecuteExtensionInput>) -> FnResult<()> {
    let args = parse_args::<MigrateTurborepoExtensionArgs>(&input.args)?;
    let workspace_root = &input.context.workspace_root;
    let mut migrator = TurboMigrator::new(&input.context, args.bun)?;

    // Migrate the workspace root config first
    let root_config_path = workspace_root.join("turbo.json");

    if root_config_path.exists() {
        host_log!(stdout, "Migrating root config <file>turbo.json</file>",);

        migrator.migrate_root_config(json::read_file(&root_config_path)?)?;

        fs::remove(root_config_path)?;
    }

    // Then migrate project configs
    for project_config_path in
        glob::walk_files(workspace_root, ["**/*/turbo.json", "!**/node_modules/**/*"])?
    {
        let rel_config_path = project_config_path.strip_prefix(workspace_root).unwrap();
        let project_source = rel_config_path.parent().unwrap().to_string_lossy();

        host_log!(
            stdout,
            "Migrating project config <file>{}</file>",
            rel_config_path.display()
        );

        migrator.migrate_project_config(&project_source, json::read_file(&project_config_path)?)?;

        fs::remove(project_config_path)?;
    }

    // Write the new config files
    migrator.inner.save_configs()?;

    host_log!(stdout, "Successfully migrated from Turborepo to moon!");

    Ok(())
}
