use crate::config::TypeScriptConfig;
use moon_pdk::VirtualPath;
use moon_pdk_api::MoonContext;
use moon_project::ProjectFragment;
use std::path::PathBuf;
use typescript_tsconfig_json::CompilerPath;

#[derive(Debug)]
pub struct TypeScriptContext {
    pub root_config: VirtualPath,
    pub root_options_config: VirtualPath,
    pub project_config: VirtualPath,
    pub workspace_root: VirtualPath,
}

fn create_virtual_path(base: &VirtualPath, path: PathBuf) -> VirtualPath {
    match base {
        VirtualPath::OnlyReal(_) => VirtualPath::OnlyReal(path),
        VirtualPath::WithReal {
            virtual_prefix,
            real_prefix,
            ..
        } => VirtualPath::WithReal {
            path,
            virtual_prefix: virtual_prefix.to_owned(),
            real_prefix: real_prefix.to_owned(),
        },
    }
}

pub fn create_typescript_context(
    base: &MoonContext,
    config: &TypeScriptConfig,
    project: &ProjectFragment,
) -> TypeScriptContext {
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

    TypeScriptContext {
        root_config: create_virtual_path(&base.workspace_root, root_config),
        root_options_config: create_virtual_path(&base.workspace_root, root_options_config),
        project_config: create_virtual_path(&base.workspace_root, project_config),
        workspace_root: base.workspace_root.clone(),
    }
}
