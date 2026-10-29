use crate::turbo_json::*;
use extension_common::migrator::*;
use moon_common::Id;
use moon_config::{
    FilePath, Input, Output, PartialProjectWorkspaceConfig,
    PartialProjectWorkspaceInheritedTasksConfig, PartialTaskArgs, PartialTaskConfig,
    PartialTaskDependency, PartialTaskDependencyConfig, PartialTaskOptionEnvFile,
    PartialTaskOptionsConfig, PartialWorkspaceProjects, TaskDependencyType, TaskOptionCache,
    TaskOutputStyle, TaskPreset,
};
use moon_pdk::*;
use moon_pdk_api::MoonContext;
use moon_target::Target;
use rustc_hash::FxHashSet;
use starbase_utils::{fs, glob, json};
use std::collections::BTreeMap;
use std::str::FromStr;

const TURBO_DEFAULT: &str = "$TURBO_DEFAULT$";
const TURBO_EXTENDS: &str = "$TURBO_EXTENDS$";
const TURBO_ROOT: &str = "$TURBO_ROOT$/";

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

    fn parse_target(&self, dep: &str) -> AnyResult<Target> {
        // ^:task
        if let Some(dep) = dep.strip_prefix('^') {
            return Target::parse(format!("^:{dep}").as_str()).map_err(map_miette_error);
        }

        // project:task
        if dep.contains('#') {
            let (package, script) = self.find_project_task_from_script(dep)?;
            let task_id = create_id(&script)?;

            return Target::parse(format!("{}:{task_id}", package.id).as_str())
                .map_err(map_miette_error);
        }

        // ~:task
        Target::parse(dep).map_err(map_miette_error)
    }

    fn find_project_in_packages(&self, package_name: &str) -> AnyResult<&PackageEntry> {
        for package in &self.packages {
            if package.id == package_name || package.alias == package_name {
                return Ok(package);
            }
        }

        Err(anyhow!(
            "Unable to migrate task as package <id>{package_name}</id> does not exist. Is it within the workspace?"
        ))
    }

    pub fn migrate_root_config(&mut self, mut turbo_json: TurboJson) -> AnyResult<()> {
        let mut implicit_inputs = vec![];
        let global = turbo_json.global.take().unwrap_or_default();

        // Globs are relative to the root, not the project
        for glob in turbo_json
            .global_dependencies
            .take()
            .into_iter()
            .chain(global.inputs)
            .chain(turbo_json.global_dot_env.take())
            .flatten()
        {
            implicit_inputs.push(Input::parse(match glob.strip_prefix('!') {
                Some(glob) => format!("!/{glob}"),
                None => format!("/{glob}"),
            })?);
        }

        for env in turbo_json
            .global_env
            .take()
            .into_iter()
            .chain(global.env)
            .flatten()
        {
            implicit_inputs.push(parse_env_input(&env));
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
        // package.json script names to turbo tasks
        if let Some(pipeline) = turbo_json.tasks.or(turbo_json.pipeline) {
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

                let task_id = create_id(&script_name)?;

                // Don't inherit the task, and only create a fresh one if configured
                if task.extends == Some(false) {
                    self.inner
                        .load_project_config(&project_source)?
                        .workspace
                        .get_or_insert(PartialProjectWorkspaceConfig::default())
                        .inherited_tasks
                        .get_or_insert(PartialProjectWorkspaceInheritedTasksConfig::default())
                        .exclude
                        .get_or_insert(vec![])
                        .push(task_id.clone());

                    if task
                        == (TurboTask {
                            extends: Some(false),
                            ..Default::default()
                        })
                    {
                        continue;
                    }
                }

                let task = self.migrate_task(task, &script_name)?;

                self.inner
                    .load_project_config(&project_source)?
                    .tasks
                    .get_or_insert(BTreeMap::default())
                    .insert(task_id, task);
            }
        }

        // Tags
        if let Some(project_source) = from_source
            && let Some(tags) = turbo_json.tags
            && !tags.is_empty()
        {
            let mut tag_ids = vec![];

            for tag in tags {
                tag_ids.push(Id::new(tag)?);
            }

            self.inner.load_project_config(project_source)?.tags = Some(tag_ids);
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
        config.description = turbo_task.description.clone();

        // Dependencies
        let mut deps = vec![];

        for dep in without_extends(&turbo_task.depends_on) {
            // $ENV input
            if let Some(env) = dep.strip_prefix('$') {
                inputs.push(Input::EnvVar(env.into()));
                continue;
            }

            deps.push(PartialTaskDependency::Target(self.parse_target(dep)?));
        }

        // Ran alongside the task
        for dep in without_extends(&turbo_task.with) {
            deps.push(PartialTaskDependency::Object(PartialTaskDependencyConfig {
                target: Some(self.parse_target(dep)?),
                type_of: Some(TaskDependencyType::Wait),
                ..Default::default()
            }));
        }

        if !deps.is_empty() {
            config.deps = Some(deps);
        }

        // Inputs
        for env in without_extends(&turbo_task.env) {
            inputs.push(parse_env_input(env));
        }

        for input in turbo_task.inputs.iter().flatten() {
            match input {
                TurboTaskInput::Glob(glob) => {
                    if glob != TURBO_EXTENDS {
                        inputs.push(parse_input(glob)?);
                    }
                }
                // moon hashes inputs right before running, so `jit` is the same as `startup`,
                // and a dependency's outputs are already covered by its hash
                TurboTaskInput::Structured(input) => {
                    if input.mode == TurboInputMode::DependencyOutputs {
                        continue;
                    }

                    if input.with_defaults == Some(true) {
                        inputs.push(parse_input(TURBO_DEFAULT)?);
                    }

                    for glob in input.globs.iter().flatten() {
                        inputs.push(parse_input(glob)?);
                    }
                }
            }
        }

        if !inputs.is_empty() {
            config.inputs = Some(inputs);
        }

        // Outputs
        let mut outputs = vec![];

        for output in without_extends(&turbo_task.outputs) {
            let output = output.replace(TURBO_ROOT, "/");

            if output.ends_with("/**") {
                outputs.push(Output::parse(format!("{output}/*"))?);
            } else {
                outputs.push(Output::parse(output)?);
            }
        }

        if !outputs.is_empty() {
            config.outputs = Some(outputs);
        }

        // Tags
        let mut tags = vec![];

        for tag in without_extends(&turbo_task.tags) {
            tags.push(Id::new(tag)?);
        }

        if !tags.is_empty() {
            config.tags = Some(tags);
        }

        // Options
        if turbo_task.cache == Some(false) {
            config
                .options
                .get_or_insert(PartialTaskOptionsConfig::default())
                .cache = turbo_task.cache.map(TaskOptionCache::Enabled);
        }

        if let Some(dot_env) = &turbo_task.dot_env {
            config
                .options
                .get_or_insert(PartialTaskOptionsConfig::default())
                .env_file = Some(if dot_env.len() == 1 {
                PartialTaskOptionEnvFile::File(FilePath::from_str(&dot_env[0])?)
            } else {
                PartialTaskOptionEnvFile::Enabled(true)
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

// Appending to inherited values is moon's default merge strategy
fn without_extends(list: &Option<Vec<String>>) -> impl Iterator<Item = &String> {
    list.iter().flatten().filter(|item| *item != TURBO_EXTENDS)
}

fn parse_env_input(env: &str) -> Input {
    if env.contains('!') || env.contains('*') {
        Input::EnvVarGlob(env.into())
    } else {
        Input::EnvVar(env.into())
    }
}

fn parse_input(glob: &str) -> AnyResult<Input> {
    // All files in the project
    if glob == TURBO_DEFAULT {
        return Ok(Input::parse("**/*")?);
    }

    Ok(Input::parse(glob.replace(TURBO_ROOT, "/"))?)
}
