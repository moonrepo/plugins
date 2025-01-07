use moon_pdk::config_struct;
use schematic::Schematic;

config_struct!(
    /// Configures and enables the Node.js platform.
    /// Docs: https://moonrepo.dev/docs/config/toolchain#node
    #[derive(Default, Schematic)]
    pub struct NodeConfig {
        /// When `version` is defined, syncs the version as a constraint to
        /// `package.json` engines.
        #[schema(default = true)]
        pub add_engines_constraint: bool,

        /// Arguments to automatically pass to all tasks that execute the
        /// `node` binary.
        pub bin_exec_args: Vec<String>,

        /// Automatically dedupes the lockfile when dependencies have changed.
        #[schema(default = true)]
        pub dedupe_on_lockfile_change: bool,

        /// Automatically infer moon tasks from `package.json` scripts.
        pub infer_tasks_from_scripts: bool,

        /// The relative root of the packages workspace. Defaults to moon's
        /// workspace root, but should be defined when nested.
        #[schema(default = ".", skip)]
        pub packages_root: String,

        /// Assumes only the root `package.json` is used for dependencies.
        /// Can be used to support the "one version policy" pattern.
        pub root_package_only: bool,

        /// Automatically syncs the configured package manager version
        /// to the root `packageManager` field in `package.json`.
        #[schema(default = true)]
        pub sync_package_manager_field: bool,

        /// Automatically syncs moon project-to-project relationships as
        /// dependencies for each `package.json` in the workspace.
        #[schema(default = true)]
        pub sync_project_workspace_dependencies: bool,
    }
);
