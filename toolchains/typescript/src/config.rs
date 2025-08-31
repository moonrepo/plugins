use moon_pdk_api::config_struct;
use schematic::Schematic;

config_struct!(
    /// Configures and enables the TypeScript toolchain.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#typescript
    #[derive(Schematic)]
    pub struct TypeScriptToolchainConfig {
        /// When `syncProjectReferences` is enabled, will create a `tsconfig.json`
        /// in referenced projects if it does not exist.
        #[schema(default = true)]
        pub create_missing_config: bool,

        /// Appends sources of project reference to `include` in `tsconfig.json`,
        /// for each project.
        pub include_project_reference_sources: bool,

        /// Appends shared types to `include` in `tsconfig.json`, for each project.
        /// If the shared types are a project and `syncProjectReferences` is enabled,
        /// they will also be synced as a project reference.
        ///
        /// Shared types must exist in a "types" folder, relative from the configured
        /// `root`.
        pub include_shared_types: bool,

        /// Name of the `tsconfig.json` file within each project.
        #[schema(default = "tsconfig.json")]
        pub project_config_file_name: String,

        /// The relative root to the TypeScript root. Primarily used for
        /// resolving project references.
        #[schema(default = ".")]
        pub root: String,

        /// Name of the `tsconfig.json` file at the workspace root.
        #[schema(default = "tsconfig.json")]
        pub root_config_file_name: String,

        /// Name of the shared compiler options `tsconfig.json` file
        /// at the workspace root.
        #[schema(default = "tsconfig.options.json")]
        pub root_options_config_file_name: String,

        /// Updates and routes `outDir` in `tsconfig.json` to moon's cache,
        /// for each project.
        pub route_out_dir_to_cache: bool,

        /// Syncs all project dependencies as `references` in `tsconfig.json`,
        /// for each project.
        #[schema(default = true)]
        pub sync_project_references: bool,

        /// Syncs all project dependencies as `paths` in `tsconfig.json`,
        /// for each project.
        pub sync_project_references_to_paths: bool,
    }
);

impl Default for TypeScriptToolchainConfig {
    fn default() -> Self {
        Self {
            create_missing_config: true,
            include_project_reference_sources: false,
            include_shared_types: false,
            project_config_file_name: "tsconfig.json".into(),
            root: ".".into(),
            root_config_file_name: "tsconfig.json".into(),
            root_options_config_file_name: "tsconfig.options.json".into(),
            route_out_dir_to_cache: false,
            sync_project_references: true,
            sync_project_references_to_paths: false,
        }
    }
}
