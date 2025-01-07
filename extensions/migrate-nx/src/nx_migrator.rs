use crate::nx_json::*;
use crate::nx_project_json::*;
use extension_common::migrator::*;
use moon_common::Id;
use moon_config::{
    FilePath, InputPath, OneOrMany, OutputPath, PartialProjectDependsOn,
    PartialProjectMetadataConfig, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PartialTaskOptionsConfig, PartialVcsConfig, PartialWorkspaceProjects, PortablePath,
    ProjectType, TaskOptionEnvFile,
};
use moon_pdk::{map_miette_error, AnyResult, MoonContext};
use moon_target::Target;
use rustc_hash::FxHashMap;
use starbase_utils::json::JsonValue;
use std::collections::BTreeMap;
use std::str::FromStr;

pub struct NxMigrator {
    pub inner: Migrator,
    pub package_manager: String,
}

impl NxMigrator {
    pub fn new(context: &MoonContext, bun: bool) -> AnyResult<Self> {
        let mut migrator = Migrator::new(&context.workspace_root)?;

        if bun {
            migrator.toolchain = Id::raw("bun");
        }

        Ok(Self {
            package_manager: migrator.detect_package_manager(),
            inner: migrator,
        })
    }

    pub fn use_default_settings(&mut self) -> AnyResult<()> {
        let tasks_config = self.inner.load_tasks_platform_config()?;

        if tasks_config.file_groups.is_none() {
            let file_groups = tasks_config.file_groups.get_or_insert(FxHashMap::default());

            if !file_groups.contains_key("default") {
                file_groups.insert(
                    Id::raw("default"),
                    vec![InputPath::ProjectGlob("**/*".into())],
                );
            }

            if !file_groups.contains_key("production") {
                file_groups.insert(Id::raw("production"), vec![]);
            }

            if !file_groups.contains_key("sharedGlobals") {
                file_groups.insert(Id::raw("sharedGlobals"), vec![]);
            }
        }

        let workspace_config = self.inner.load_workspace_config()?;

        if workspace_config.projects.is_none() {
            workspace_config.projects = Some(PartialWorkspaceProjects::Globs(vec![
                "apps/*".into(),
                "packages/*".into(),
            ]));
        }

        Ok(())
    }

    pub fn migrate_root_config(&mut self, nx_json: NxJson) -> AnyResult<()> {
        if let Some(default_branch) = nx_json
            .default_base
            .or_else(|| nx_json.affected.and_then(|aff| aff.default_base))
        {
            self.inner
                .load_workspace_config()?
                .vcs
                .get_or_insert(PartialVcsConfig::default())
                .default_branch = Some(default_branch);
        }

        if let Some(named_inputs) = nx_json.named_inputs {
            if !named_inputs.is_empty() {
                let file_groups = self
                    .inner
                    .load_tasks_platform_config()?
                    .file_groups
                    .get_or_insert(FxHashMap::default());

                for (name, raw_inputs) in named_inputs {
                    let group = migrate_inputs(&raw_inputs, true)?;

                    if !group.is_empty() {
                        file_groups.insert(create_id(name)?, group);
                    }
                }
            }
        }

        if let Some(target_defaults) = nx_json.target_defaults {
            let tasks = self
                .inner
                .load_tasks_platform_config()?
                .tasks
                .get_or_insert(BTreeMap::default());

            for (name, target_config) in target_defaults {
                tasks.insert(
                    create_id(name)?,
                    migrate_task(&target_config, &self.package_manager)?,
                );
            }
        }

        if let Some(layout) = nx_json.workspace_layout {
            let workspace_config = self.inner.load_workspace_config()?;

            if workspace_config.projects.is_none() {
                workspace_config.projects = Some(PartialWorkspaceProjects::Globs(vec![
                    format!("{}/*", layout.apps_dir.unwrap_or("apps".into())),
                    format!("{}/*", layout.libs_dir.unwrap_or("libs".into())),
                ]));
            }
        }

        Ok(())
    }

    pub fn migrate_workspace_config(&mut self, workspace_json: NxWorkspaceJson) -> AnyResult<()> {
        let mut projects = FxHashMap::default();

        for (id, source) in workspace_json.projects {
            projects.insert(create_id(id)?, source);
        }

        if !projects.is_empty() {
            self.inner.load_workspace_config()?.projects =
                Some(PartialWorkspaceProjects::Sources(projects));
        }

        Ok(())
    }

    pub fn migrate_project_config(
        &mut self,
        project_source: &str,
        project_json: NxProjectJson,
    ) -> AnyResult<()> {
        let config = self.inner.load_project_config(project_source)?;

        if let Some(name) = project_json.name {
            config.id = Some(create_id(name)?);
        }

        if let Some(implicit_dependencies) = project_json.implicit_dependencies {
            if !implicit_dependencies.is_empty() {
                let depends_on = config.depends_on.get_or_insert(vec![]);

                for dep in implicit_dependencies {
                    depends_on.push(PartialProjectDependsOn::String(create_id(dep)?));
                }
            }
        }

        if let Some(named_inputs) = project_json.named_inputs {
            if !named_inputs.is_empty() {
                let file_groups = config.file_groups.get_or_insert(FxHashMap::default());

                for (name, raw_inputs) in named_inputs {
                    let group = migrate_inputs(&raw_inputs, true)?;

                    if !group.is_empty() {
                        file_groups.insert(create_id(name)?, group);
                    }
                }
            }
        }

        if let Some(project_type) = project_json.project_type {
            if project_type == "library" || project_type == "lib" {
                config.type_of = Some(ProjectType::Library);
            } else if project_type == "application" || project_type == "app" {
                config.type_of = Some(ProjectType::Application);
            }
        }

        if let Some(raw_tags) = project_json.tags {
            let tags = config.tags.get_or_insert(vec![]);

            for tag in raw_tags {
                tags.push(create_id(tag)?);
            }
        }

        if let Some(targets) = project_json.targets {
            let tasks = config.tasks.get_or_insert(BTreeMap::default());

            for (name, target) in targets {
                let task_id = create_id(name)?;

                tasks.insert(
                    task_id.clone(),
                    migrate_task(&target, &self.package_manager)?,
                );

                // https://nx.dev/concepts/executors-and-configurations#use-task-configurations
                if let Some(configurations) = target.configurations {
                    for (config_name, config_options) in configurations {
                        tasks.insert(
                            create_id(format!("{task_id}.{config_name}"))?,
                            PartialTaskConfig {
                                extends: Some(task_id.clone()),
                                args: Some(PartialTaskArgs::List(migrate_options_to_args(
                                    &config_options,
                                ))),
                                ..PartialTaskConfig::default()
                            },
                        );
                    }
                }
            }
        }

        if let Some(metadata) = project_json.metadata {
            config
                .project
                .get_or_insert(PartialProjectMetadataConfig::default())
                .metadata
                .get_or_insert(Default::default())
                .extend(metadata);
        }

        Ok(())
    }

    pub fn migrate_project_package_config(
        &mut self,
        project_source: &str,
        nx_package_json: PackageJsonWithNx,
    ) -> AnyResult<()> {
        if let Some(nx) = nx_package_json.nx {
            self.migrate_project_config(project_source, nx)?;
        }

        Ok(())
    }
}

fn is_path_or_glob(value: &str) -> bool {
    (value.contains('/')
        || value.contains('*')
        || value.contains('.')
        || value.contains('{')
        || value.starts_with('!'))
        && !value.starts_with("http")
        && !value.contains(' ')
}

fn replace_tokens(value: &str, for_sources: bool) -> String {
    let mut result = value.replace("{projectName}", "$project");

    if for_sources {
        if result.starts_with("!{projectRoot}/") {
            result = result.replacen("!{projectRoot}/", "!", 1);
        }

        if result.starts_with("{projectRoot}/") {
            result = result.replacen("{projectRoot}/", "", 1);
        }
    }

    result = result.replace("{projectRoot}", "$projectRoot");

    if for_sources {
        if result.starts_with("!{workspaceRoot}/") {
            result = result.replacen("!{workspaceRoot}/", "!/", 1);
        }

        if result.starts_with("{workspaceRoot}/") {
            result = result.replacen("{workspaceRoot}/", "/", 1);
        }
    }

    result = result.replace("{workspaceRoot}", "$workspaceRoot");

    result
}

fn migrate_inputs(raw_inputs: &[NxInput], for_file_groups: bool) -> AnyResult<Vec<InputPath>> {
    let mut inputs = vec![];

    for input in raw_inputs {
        match input {
            NxInput::Dep { .. } => {
                // Not supported
            }
            NxInput::DepOutput { .. } => {
                // Not supported
            }
            NxInput::External { .. } => {
                // Not supported, moon parses lockfiles automatically
            }
            NxInput::Env { env } => {
                inputs.push(InputPath::EnvVar(env.to_owned()));
            }
            NxInput::Fileset { fileset } => {
                inputs.push(InputPath::from_str(&replace_tokens(fileset, true))?);
            }
            NxInput::Runtime { .. } => {
                // Not supported, moon includes tool version automatically
            }
            NxInput::Source(source) => {
                // File path or glob
                if is_path_or_glob(source) {
                    inputs.push(InputPath::from_str(&replace_tokens(source, true))?);
                }
                // Named input
                else if !source.starts_with('^') && !for_file_groups {
                    inputs.push(InputPath::TokenFunc(format!("@group({source})")));
                }
            }
        };
    }

    Ok(inputs)
}

fn convert_value_to_string_without_quotes(value: &JsonValue) -> String {
    value
        .to_string()
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_owned()
}

// We'll parse these arguments back into an object using `yargs-parser`:
// https://www.npmjs.com/package/yargs-parser
fn migrate_options_to_args(options: &FxHashMap<String, JsonValue>) -> Vec<String> {
    let mut args = vec![];

    for (key, value) in options {
        if matches!(value, JsonValue::Null) {
            continue;
        }

        match value {
            JsonValue::Bool(value) => {
                if *value {
                    args.push(format!("--{key}"));
                } else {
                    args.push(format!("--no-{key}"));
                }
            }
            JsonValue::String(value) => {
                args.push(format!("--{key}"));

                // All Nx options are workspace relative, not project relative,
                // so let's prepend with a token to ensure that it works!
                let value = replace_tokens(value, false);

                args.push(if is_path_or_glob(&value) && !value.starts_with('$') {
                    format!("$workspaceRoot/{}", value.trim_start_matches('/'))
                } else {
                    value
                });
            }
            inner => {
                args.push(format!("--{key}"));
                args.push(convert_value_to_string_without_quotes(inner));
            }
        }
    }

    args
}

fn inject_args_into_task(nx_target: &NxTargetOptions, config: &mut PartialTaskConfig) {
    // This is a bit complicated, since moon doesn't have a concept of arbitrary
    // options, but in Nx, options can also be passed as command line args,
    // so let's replicate that and hope it works correctly!
    if let Some(options) = &nx_target.options {
        let args = migrate_options_to_args(options);

        if !args.is_empty() {
            config.args = Some(PartialTaskArgs::List(args));
        }
    }
}

// https://nx.dev/nx-api/nx/executors/noop
fn migrate_noop_task(nx_target: &NxTargetOptions) -> AnyResult<PartialTaskConfig> {
    let mut config = PartialTaskConfig {
        command: Some(PartialTaskArgs::String("noop".into())),
        ..PartialTaskConfig::default()
    };

    inject_args_into_task(nx_target, &mut config);

    Ok(config)
}

// https://nx.dev/nx-api/nx/executors/run-commands
fn migrate_run_commands_task(nx_target: &NxTargetOptions) -> AnyResult<PartialTaskConfig> {
    let mut config = PartialTaskConfig {
        toolchain: Some(OneOrMany::One(Id::raw("system"))),
        ..PartialTaskConfig::default()
    };

    // https://nx.dev/nx-api/nx/executors/run-commands#options
    if let Some(options) = &nx_target.options {
        if let Some(JsonValue::String(command)) = options.get("command") {
            config.command = Some(PartialTaskArgs::String(command.to_owned()));
        } else if let Some(JsonValue::Array(commands)) = options.get("commands") {
            config.command = Some(PartialTaskArgs::String(
                commands
                    .iter()
                    .map(convert_value_to_string_without_quotes)
                    .collect::<Vec<_>>()
                    .join(" && "),
            ));
        }

        if let Some(JsonValue::String(cwd)) = options.get("cwd") {
            config
                .env
                .get_or_insert(FxHashMap::default())
                .insert("CWD".into(), cwd.to_owned());
        }

        if let Some(JsonValue::Object(envs)) = options.get("env") {
            let env = config.env.get_or_insert(FxHashMap::default());

            for (key, value) in envs {
                env.insert(
                    key.to_owned(),
                    convert_value_to_string_without_quotes(value),
                );
            }
        }

        if let Some(JsonValue::String(env_file)) = options.get("envFile") {
            config
                .options
                .get_or_insert(PartialTaskOptionsConfig::default())
                .env_file = Some(TaskOptionEnvFile::File(FilePath::from_str(env_file)?));
        }
    }

    if config.command.is_none() {
        config.command = Some(PartialTaskArgs::String("noop".into()));
    }

    Ok(config)
}

// https://nx.dev/nx-api/nx/executors/run-script
fn migrate_run_script_task(
    nx_target: &NxTargetOptions,
    package_manager: &str,
) -> AnyResult<PartialTaskConfig> {
    let mut config = PartialTaskConfig::default();

    if let Some(options) = &nx_target.options {
        if let Some(JsonValue::String(script)) = options.get("script") {
            config.command = Some(PartialTaskArgs::String(format!(
                "{package_manager} run {script}"
            )));
        }
    }

    Ok(config)
}

fn migrate_task(
    nx_target: &NxTargetOptions,
    package_manager: &str,
) -> AnyResult<PartialTaskConfig> {
    let mut inject_args = false;

    let mut config = if let Some(executor) = &nx_target.executor {
        if executor == "nx:noop" {
            migrate_noop_task(nx_target)?
        } else if executor == "nx:run-commands" {
            migrate_run_commands_task(nx_target)?
        } else if executor == "nx:run-script" {
            migrate_run_script_task(nx_target, package_manager)?
        } else {
            let mut parts = executor.splitn(2, ':');
            let mut package = parts.next().unwrap_or_default();
            let target = parts.next().unwrap_or_default();

            if let Some(index) = package.find('/') {
                package = &package[index + 1..];
            }

            inject_args = true;

            PartialTaskConfig {
                command: Some(PartialTaskArgs::String(if package == target {
                    target.to_owned()
                } else {
                    format!("{package} {target}")
                })),
                ..PartialTaskConfig::default()
            }
        }
    } else {
        let mut config = PartialTaskConfig::default();

        if let Some(command) = &nx_target.command {
            config.command = Some(PartialTaskArgs::String(command.to_owned()));
        }

        inject_args = true;
        config
    };

    // Arguments
    if inject_args {
        inject_args_into_task(nx_target, &mut config);
    }

    // Dependencies
    // - https://nx.dev/reference/project-configuration#dependson
    if let Some(depends_on) = &nx_target.depends_on {
        let mut deps: Vec<Target> = vec![];

        for dep in depends_on {
            match dep {
                NxDependsOn::Object {
                    dependencies,
                    target,
                    projects,
                    ..
                } => {
                    if let Some(projects) = projects {
                        match projects {
                            StringOrList::List(ids) => {
                                for id in ids {
                                    deps.push(Target::new(id, target).map_err(map_miette_error)?);
                                }
                            }
                            StringOrList::String(scope) => {
                                deps.push(
                                    Target::parse(
                                        format!(
                                            "{}:{target}",
                                            if scope == "self" { "~" } else { "^" }
                                        )
                                        .as_str(),
                                    )
                                    .map_err(map_miette_error)?,
                                );
                            }
                        };
                    } else if let Some(dependencies) = dependencies {
                        if *dependencies {
                            deps.push(
                                Target::parse(format!("^:{target}").as_str())
                                    .map_err(map_miette_error)?,
                            );
                        }
                    } else {
                        deps.push(Target::new_self(target).map_err(map_miette_error)?);
                    }
                }
                NxDependsOn::String(target) => {
                    if let Some(target) = target.strip_prefix('^') {
                        deps.push(
                            Target::parse(format!("^:{target}").as_str())
                                .map_err(map_miette_error)?,
                        );
                    } else {
                        deps.push(Target::new_self(target).map_err(map_miette_error)?);
                    }
                }
            };
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
    // - https://nx.dev/recipes/running-tasks/configure-inputs
    // - https://nx.dev/reference/inputs
    let mut inputs = vec![];

    if let Some(raw_inputs) = &nx_target.inputs {
        inputs.extend(migrate_inputs(raw_inputs, false)?);
    }

    if !inputs.is_empty() {
        config.inputs = Some(inputs);
    }

    // Outputs
    // - https://nx.dev/recipes/running-tasks/configure-outputs
    if let Some(raw_outputs) = &nx_target.outputs {
        let mut outputs = vec![];

        for output in raw_outputs {
            outputs.push(OutputPath::from_str(&replace_tokens(output, true))?);
        }

        if !outputs.is_empty() {
            config.outputs = Some(outputs);
        }
    }

    if nx_target.cache == Some(true) {
        config
            .options
            .get_or_insert(PartialTaskOptionsConfig::default())
            .cache = nx_target.cache;
    }

    Ok(config)
}
