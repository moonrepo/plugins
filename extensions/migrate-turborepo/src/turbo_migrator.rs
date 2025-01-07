use crate::turbo_json::*;
use extension_common::migrator::*;
use moon_common::Id;
use moon_config::{
    FilePath, InputPath, OutputPath, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PartialTaskOptionsConfig, PartialWorkspaceProjects, PortablePath, TaskOptionEnvFile,
    TaskOutputStyle, TaskPreset,
};
use moon_pdk::*;
use moon_target::Target;
use rustc_hash::FxHashSet;
use starbase_utils::glob;
use starbase_utils::{fs, json};
use std::collections::BTreeMap;
use std::str::FromStr;

pub struct PackageEntry {
    id: Id,
    alias: String,
    source: String,
}

pub struct TurboMigrator {
    pub inner: Migrator,
    pub package_manager: String,
    pub package_globs: FxHashSet<String>,
    pub packages: Vec<PackageEntry>,
}

impl TurboMigrator {
    pub fn new(context: &MoonContext, bun: bool) -> AnyResult<Self> {
        let mut migrator = Migrator::new(&context.workspace_root)?;

        if bun {
            migrator.toolchain = Id::raw("bun");
        }

        // Load current packages
        let mut packages = vec![];
        let mut package_globs = FxHashSet::default();

        for package_json_path in glob::walk_files(
            &context.workspace_root,
            ["**/*/package.json", "!**/node_modules/**/*"],
        )? {
            let package_json: PackageJson = json::read_file(&package_json_path)?;

            if let Some(name) = package_json.name {
                let package_root = package_json_path.parent().unwrap();
                let package_source = package_root.strip_prefix(&context.workspace_root).unwrap();

                package_globs.insert(format!(
                    "{}/*",
                    package_source.parent().unwrap().to_string_lossy()
                ));

                packages.push(PackageEntry {
                    id: create_id(fs::file_name(package_root))?,
                    alias: name,
                    source: package_source.to_string_lossy().to_string(),
                });
            }
        }

        Ok(Self {
            package_manager: migrator.detect_package_manager(),
            inner: migrator,
            package_globs,
            packages,
        })
    }

    fn find_project_task_from_script(&self, script: &str) -> AnyResult<(&PackageEntry, String)> {
        let mut parts = script.split('#');
        let package_name = parts.next().unwrap();
        let task_id = parts.next().unwrap();
        let project = self.find_project_in_packages(package_name)?;

        Ok((project, task_id.to_owned()))
    }

    fn find_project_in_packages(&self, package_name: &str) -> AnyResult<&PackageEntry> {
        for package in &self.packages {
            if package.id == package_name || package.alias == package_name {
                return Ok(package);
            }
        }

        Err(anyhow!("Unable to migrate task as package <id>{package_name}</id> does not exist. Is it within the workspace?"))
    }

    pub fn migrate_root_config(&mut self, mut turbo_json: TurboJson) -> AnyResult<()> {
        let mut implicit_inputs = vec![];

        if let Some(global_deps) = turbo_json.global_dependencies.take() {
            for dep in global_deps {
                implicit_inputs.push(InputPath::from_str(&dep)?);
            }
        }

        if let Some(global_dot_env) = turbo_json.global_dot_env.take() {
            for env_file in global_dot_env {
                implicit_inputs.push(InputPath::from_str(&env_file)?);
            }
        }

        if let Some(global_env) = turbo_json.global_env.take() {
            for env in global_env {
                implicit_inputs.push(InputPath::EnvVar(env.to_owned()));
            }
        }

        if !implicit_inputs.is_empty() {
            self.inner
                .load_tasks_platform_config()?
                .implicit_inputs
                .get_or_insert(vec![])
                .extend(implicit_inputs);
        }

        if !self.package_globs.is_empty() {
            let workspace_config = self.inner.load_workspace_config()?;

            if workspace_config.projects.is_none() {
                workspace_config.projects = Some(PartialWorkspaceProjects::Globs(
                    self.package_globs.clone().into_iter().collect(),
                ));
            }
        }

        self.migrate_pipeline(turbo_json, None)?;

        Ok(())
    }

    pub fn migrate_project_config(
        &mut self,
        project_source: &str,
        turbo_json: TurboJson,
    ) -> AnyResult<()> {
        self.migrate_pipeline(turbo_json, Some(project_source))
    }

    pub fn migrate_pipeline(
        &mut self,
        turbo_json: TurboJson,
        from_source: Option<&str>,
    ) -> AnyResult<()> {
        let Some(pipeline) = turbo_json.tasks.or(turbo_json.pipeline) else {
            return Ok(());
        };

        // package.json script names to turbo tasks
        for (script, task) in pipeline {
            let project_source;
            let script_name;

            // Root-level task
            if let Some(root_script) = script.strip_prefix("//#") {
                project_source = String::new();
                script_name = root_script.to_owned();
            }
            // Project-scoped task
            else if script.contains('#') {
                (project_source, script_name) = self
                    .find_project_task_from_script(&script)
                    .map(|(p, i)| (p.source.to_owned(), i))?;
            }
            // For a source task
            else if let Some(source) = from_source {
                project_source = source.to_owned();
                script_name = script;
            }
            // Global task
            else {
                let task = self.migrate_task(task, &script)?;
                let task_id = create_id(&script)?;

                self.inner
                    .load_tasks_platform_config()?
                    .tasks
                    .get_or_insert(BTreeMap::default())
                    .insert(task_id, task);

                continue;
            }

            let task = self.migrate_task(task, &script_name)?;
            let task_id = create_id(&script_name)?;

            self.inner
                .load_project_config(&project_source)?
                .tasks
                .get_or_insert(BTreeMap::default())
                .insert(task_id, task);
        }

        Ok(())
    }

    pub fn migrate_task(
        &self,
        turbo_task: TurboTask,
        package_script: &str,
    ) -> AnyResult<PartialTaskConfig> {
        let mut config = PartialTaskConfig::default();
        let mut inputs = vec![];

        config.command = Some(PartialTaskArgs::String(format!(
            "{} run {}",
            self.package_manager, package_script
        )));

        // Dependencies
        if let Some(depends_on) = &turbo_task.depends_on {
            let mut deps: Vec<Target> = vec![];

            for dep in depends_on {
                // $ENV input
                if let Some(env) = dep.strip_prefix('$') {
                    inputs.push(InputPath::EnvVar(env.into()));
                    continue;
                }

                // ^:task
                if let Some(dep) = dep.strip_prefix('^') {
                    deps.push(
                        Target::parse(format!("^:{dep}").as_str()).map_err(map_miette_error)?,
                    );
                    continue;
                }

                // project:task
                if dep.contains('#') {
                    let (package, script) = self.find_project_task_from_script(dep)?;
                    let task_id = create_id(&script)?;

                    deps.push(
                        Target::parse(format!("{}:{task_id}", package.id).as_str())
                            .map_err(map_miette_error)?,
                    );
                    continue;
                }

                // ~:task
                deps.push(Target::parse(dep).map_err(map_miette_error)?);
            }

            if !deps.is_empty() {
                config.deps = Some(
                    deps.into_iter()
                        .map(PartialTaskDependency::Target)
                        .collect(),
                );
            }
        }

        // Inputs
        if let Some(env_vars) = &turbo_task.env {
            for env in env_vars {
                inputs.push(InputPath::EnvVar(env.into()));
            }
        }

        if let Some(raw_inputs) = &turbo_task.inputs {
            for input in raw_inputs {
                if input == "$TURBO_DEFAULT$" {
                    continue;
                }

                inputs.push(InputPath::from_str(input)?);
            }
        }

        if !inputs.is_empty() {
            config.inputs = Some(inputs);
        }

        // Outputs
        if let Some(raw_outputs) = &turbo_task.outputs {
            let mut outputs = vec![];

            for output in raw_outputs {
                if output.ends_with("/**") {
                    outputs.push(OutputPath::ProjectGlob(format!("{output}/*")));
                } else {
                    outputs.push(OutputPath::from_str(output)?);
                }
            }

            if !outputs.is_empty() {
                config.outputs = Some(outputs);
            }
        }

        // Options
        if turbo_task.cache == Some(false) {
            config
                .options
                .get_or_insert(PartialTaskOptionsConfig::default())
                .cache = turbo_task.cache;
        }

        if let Some(dot_env) = &turbo_task.dot_env {
            config
                .options
                .get_or_insert(PartialTaskOptionsConfig::default())
                .env_file = Some(if dot_env.len() == 1 {
                TaskOptionEnvFile::File(FilePath::from_str(&dot_env[0])?)
            } else {
                TaskOptionEnvFile::Enabled(true)
            });
        }

        if let Some(output_mode) = turbo_task.output_logs.or(turbo_task.output_mode) {
            let output_style = match output_mode {
                TurboOutputMode::HashOnly => Some(TaskOutputStyle::Hash),
                TurboOutputMode::NewOnly => Some(TaskOutputStyle::Buffer),
                TurboOutputMode::ErrorsOnly => Some(TaskOutputStyle::BufferOnlyFailure),
                _ => None,
            };

            if output_style.is_some() {
                config
                    .options
                    .get_or_insert(PartialTaskOptionsConfig::default())
                    .output_style = output_style;
            }
        }

        if turbo_task.interactive == Some(true) {
            config
                .options
                .get_or_insert(PartialTaskOptionsConfig::default())
                .interactive = turbo_task.interactive;
        }

        if turbo_task.persistent == Some(true) {
            config.preset = Some(TaskPreset::Server);
        }

        Ok(config)
    }
}
