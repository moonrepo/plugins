use crate::config::{JavaScriptPackageManager, JavaScriptToolchainConfig};
use crate::lockfiles::DenoJsonTask;
use extism_pdk::debug;
use moon_common::Id;
use moon_config::{
    OneOrMany, Output, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PartialTaskOptionsConfig, TaskOptionCache, TaskOptionRunInCI, TaskPreset,
};
use moon_pdk::{AnyResult, map_miette_error};
use moon_target::Target;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashSet};

pub struct TasksInferrer<'a> {
    config: &'a JavaScriptToolchainConfig,
    tasks: BTreeMap<Id, PartialTaskConfig>,
    life_cycles: HashSet<String>,
}

impl<'a> TasksInferrer<'a> {
    pub fn new(config: &'a JavaScriptToolchainConfig) -> TasksInferrer<'a> {
        Self {
            config,
            tasks: BTreeMap::default(),
            life_cycles: "preprepare|prepare|postprepare|prepublish|prepublishOnly|publish|postpublish|prepack|pack|postpack|preinstall|install|postinstall|preversion|version|postversion|dependencies"
                .split('|')
                .map(|lc| lc.to_string())
                .collect::<HashSet<_>>(),
        }
    }

    pub fn infer_from_deno_tasks(
        mut self,
        tasks: &BTreeMap<String, DenoJsonTask>,
    ) -> AnyResult<BTreeMap<Id, PartialTaskConfig>> {
        for (name, deno_task) in tasks {
            let command = deno_task.get_command();
            let deps = deno_task.get_dependencies();
            let args = split_script(name, command);

            // Deno has no lifecycle scripts or pre and post hooks, so only skip
            // tasks without a command or dependencies, like npm's `is_valid`
            if (command.is_empty() && deps.is_empty()) || runs_moon(&args) {
                continue;
            }

            let Some(id) = self.create_task_id(name) else {
                continue;
            };

            let mut task = self.create_task(name, &args, "deno.json task")?;

            // `deno task <name>` runs the dependencies itself, which moon runs as
            // tasks, so only run the command, through Deno's task shell (v2.1+)
            if command.is_empty() {
                task.command = Some(PartialTaskArgs::String("noop".into()));
            } else if !deps.is_empty() {
                task.command = Some(PartialTaskArgs::List(vec![
                    "deno".into(),
                    "task".into(),
                    "--eval".into(),
                    command.into(),
                ]));
            }

            for dep_name in deps {
                if let Some(dep_id) = self.create_task_id(dep_name) {
                    task.deps
                        .get_or_insert_default()
                        .push(PartialTaskDependency::Target(
                            Target::parse(&format!("~:{dep_id}")).map_err(map_miette_error)?,
                        ));
                }
            }

            self.insert_task(id, name, task);
        }

        Ok(self.tasks)
    }

    pub fn infer_from_package_scripts(
        mut self,
        scripts: BTreeMap<&String, &String>,
    ) -> AnyResult<BTreeMap<Id, PartialTaskConfig>> {
        for (name, script) in &scripts {
            let args = split_script(name, script);

            if !self.is_valid(name, script, &args, Some(&scripts)) {
                continue;
            }

            let Some(id) = self.create_task_id(name) else {
                continue;
            };

            let task = self.create_task(name, &args, "package.json script")?;

            self.insert_task(id, name, task);
        }

        Ok(self.tasks)
    }

    // ADR-0008.5: Skip a script whose name can't be a task ID, like `//`
    fn create_task_id(&self, name: &str) -> Option<Id> {
        match Id::clean(name) {
            Ok(id) => Some(id),
            Err(error) => {
                debug!("Skipping the script {name}, as it can't be a task ID: {error}");

                None
            }
        }
    }

    // Names like `build:js` and `build-js` clean to the same ID, so keep the first
    fn insert_task(&mut self, id: Id, name: &str, task: PartialTaskConfig) {
        match self.tasks.entry(id) {
            Entry::Occupied(entry) => {
                debug!(
                    "Skipping the script {name}, as another script already created task {}",
                    entry.key()
                );
            }
            Entry::Vacant(entry) => {
                entry.insert(task);
            }
        };
    }

    fn create_task(
        &self,
        name: &str,
        script_args: &[String],
        source: &str,
    ) -> AnyResult<PartialTaskConfig> {
        let package_manager = self.config.package_manager.unwrap_or_default();

        let mut config = PartialTaskConfig::default();
        let mut options = PartialTaskOptionsConfig::default();
        let mut modify_options = false;

        match package_manager {
            JavaScriptPackageManager::Deno => {
                config.description = Some(format!("Inherited from `{name}` {source}."));

                // command + args
                config.command = Some(PartialTaskArgs::List(vec![
                    "deno".to_string(),
                    "task".to_string(),
                    name.to_string(),
                ]));
            }
            _ => {
                config.description = Some(format!("Inherited from `{name}` {source}."));

                // command + args
                let mut command = vec![
                    package_manager.to_string(),
                    "run".to_string(),
                    name.to_string(),
                ];

                // npm reads flags after the script name as its own config,
                // so passthrough args from moon must follow `--`
                if package_manager == JavaScriptPackageManager::Npm {
                    command.push("--".into());
                }

                config.command = Some(PartialTaskArgs::List(command));
            }
        };

        // outputs
        for (index, arg) in script_args.iter().enumerate() {
            let (option, value) = if let Some((prefix, suffix)) = arg.split_once('=') {
                (prefix, Some(suffix))
            } else {
                (arg.as_str(), script_args.get(index + 1).map(|a| a.as_str()))
            };

            if self.is_output_option(option)
                && let Some(output) = value
                && let Some(output_path) = self.clean_output_path(output)
            {
                match Output::parse(&output_path) {
                    Ok(output) => config.outputs.get_or_insert_default().push(output),
                    Err(error) => {
                        debug!("Skipping the output {output_path} of script {name}: {error}");
                    }
                };
            }
        }

        // preset
        if self.is_dev_script_name(name) {
            if self.has_watch_option(script_args) {
                options.cache = Some(TaskOptionCache::Enabled(false));
                options.persistent = Some(true);
                options.run_in_ci = Some(TaskOptionRunInCI::Enabled(false));

                modify_options = true;
            } else {
                config.preset = Some(TaskPreset::Server);
            };
        }

        // toolchains
        // Bun and Deno are their own runtime. nub runs scripts on the `node`
        // on `PATH`, and moon drops `node` when it isn't configured.
        let mut toolchains = vec![Id::raw("javascript"), package_manager.get_toolchain_id()];
        let runtime = package_manager.get_runtime_toolchain();

        if !toolchains.contains(&runtime) {
            toolchains.push(runtime);
        }

        config.toolchains = Some(OneOrMany::Many(toolchains));

        // options
        if self.has_help_or_version_option(script_args) || self.has_watch_option(script_args) {
            options.run_in_ci = Some(TaskOptionRunInCI::Enabled(false));
            modify_options = true;
        }

        if modify_options {
            config.options = Some(options);
        }

        Ok(config)
    }

    fn clean_output_path(&self, output: &str) -> Option<String> {
        if output.starts_with("..")
            || output.starts_with('/')
            || output.starts_with("C:")
            || output.starts_with("D:")
        {
            None
        } else if output.starts_with("./") || output.starts_with(".\\") {
            Some(output[2..].replace("\\", "/"))
        } else {
            Some(output.replace("\\", "/"))
        }
    }

    fn has_help_or_version_option(&self, args: &[String]) -> bool {
        args.iter().any(|arg| arg == "--help" || arg == "--version")
    }

    fn has_watch_option(&self, args: &[String]) -> bool {
        let mut command = None;

        for arg in args {
            // A new command in a chain, like `npm run build && tsc -w`
            if matches!(arg.as_str(), "&&" | "||" | ";" | "|" | "&") {
                command = None;
                continue;
            }

            let Some(command_name) = command else {
                command = Some(arg.as_str());
                continue;
            };

            match arg.as_str() {
                "--watch" | "--watchAll" => return true,
                // `-w` selects a workspace for package managers
                "-w" if !matches!(command_name, "bun" | "npm" | "pnpm" | "yarn") => return true,
                _ => {}
            };
        }

        false
    }

    fn is_valid(
        &self,
        name: &str,
        script: &str,
        args: &[String],
        scripts: Option<&BTreeMap<&String, &String>>,
    ) -> bool {
        if self.life_cycles.contains(name) || script.is_empty() || runs_moon(args) {
            return false;
        }

        // Package managers only run `pre<name>` and `post<name>` as hooks
        // when a `<name>` script exists, so `prettier` or `postcss` aren't
        if let Some(scripts) = scripts {
            for prefix in ["pre", "post"] {
                if let Some(hook_for) = name.strip_prefix(prefix)
                    && scripts.keys().any(|key| *key == hook_for)
                {
                    return false;
                }
            }
        }

        true
    }

    fn is_dev_script_name(&self, name: &str) -> bool {
        for affix in ["dev", "start", "serve", "preview"] {
            if name == affix
                || name.ends_with(&format!(":{affix}"))
                || name.starts_with(&format!("{affix}:"))
            {
                return true;
            }
        }

        false
    }

    fn is_output_option(&self, arg: &str) -> bool {
        for option in [
            "-o",
            "--out",
            "--output",
            "--outfile",
            "--outdir",
            "--out-file",
            "--out-dir",
            "--dist",
        ] {
            if arg == option || arg.to_lowercase() == option {
                return true;
            }
        }

        false
    }
}

// ADR-0008.5: Detect options from a whitespace split when the script can't be
// split like a shell would, instead of skipping it
fn split_script(name: &str, script: &str) -> Vec<String> {
    shell_words::split(script).unwrap_or_else(|error| {
        debug!("Splitting the script {name} on whitespace, as it can't be parsed: {error}");

        script
            .split_whitespace()
            .map(|arg| arg.to_owned())
            .collect()
    })
}

// A script like `moon run :check` would run the task inferred from itself
fn runs_moon(args: &[String]) -> bool {
    args.iter()
        .find(|arg| !is_env_assignment(arg))
        .is_some_and(|arg| arg == "moon" || arg == "moonx")
}

fn is_env_assignment(arg: &str) -> bool {
    arg.split_once('=').is_some_and(|(key, _)| {
        !key.is_empty()
            && key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    })
}
