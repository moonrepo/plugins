use crate::config::TypeScriptToolchainConfig;
use moon_pdk::{AnyResult, VirtualPath, VirtualPathExt};
use moon_pdk_api::MoonContext;
use moon_project::ProjectFragment;
use typescript_tsconfig_json::CompilerPath;

#[derive(Debug)]
pub struct TypeScriptContext {
    pub root_config: VirtualPath,
    pub root_options_config: VirtualPath,
    pub project_config: VirtualPath,
    pub workspace_root: VirtualPath,
}

pub fn create_typescript_context(
    base: &MoonContext,
    config: &TypeScriptToolchainConfig,
    project: &ProjectFragment,
) -> AnyResult<TypeScriptContext> {
    let root_config = CompilerPath::resolve(
        base.workspace_root
            .join(&config.root)
            .join(&config.root_config_file_name)
            .to_path_buf(),
    );
    let root_options_config = CompilerPath::resolve(
        base.workspace_root
            .join(&config.root)
            .join(&config.root_options_config_file_name)
            .to_path_buf(),
    );
    let project_config = CompilerPath::resolve(
        base.workspace_root
            .join(&project.source)
            .join(&config.project_config_file_name)
            .to_path_buf(),
    );

    Ok(TypeScriptContext {
        root_config: VirtualPath::create(root_config)?,
        root_options_config: VirtualPath::create(root_options_config)?,
        project_config: VirtualPath::create(project_config)?,
        workspace_root: base.workspace_root.clone(),
    })
}
