use crate::config::{JavaScriptPackageManager, JavaScriptToolchainConfig};
use crate::lockfiles::DenoJsonTask;
use moon_common::Id;
use moon_config::{
    OneOrMany, OutputPath, PartialTaskArgs, PartialTaskConfig, PartialTaskDependency,
    PartialTaskOptionsConfig, TaskOptionRunInCI, TaskPreset,
};
use moon_pdk::{AnyResult, map_miette_error};
use moon_target::Target;
use std::collections::{BTreeMap, HashSet};

pub struct TasksInferrer<'a> {
    config: &'a JavaScriptToolchainConfig,
    tasks: BTreeMap<String, PartialTaskConfig>,
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
    ) -> AnyResult<BTreeMap<String, PartialTaskConfig>> {
        for (name, deno_task) in tasks {
            let command = deno_task.get_command();
            let deps = deno_task.get_dependencies();

            if self.is_valid(name, command) {
                let mut task = self.create_task(name, command)?;

                if !deps.is_empty() {
                    for dep_name in deps {
                        let dep_id = self.create_task_id(dep_name)?;

                        task.deps
                            .get_or_insert_default()
                            .push(PartialTaskDependency::Target(
                                Target::parse(&format!("~:{dep_id}")).map_err(map_miette_error)?,
                            ));
                    }
                }

                self.tasks.insert(self.create_task_id(name)?, task);
            }
        }

        Ok(self.tasks)
    }

    pub fn infer_from_package_scripts(
        mut self,
        scripts: BTreeMap<&String, &String>,
    ) -> AnyResult<BTreeMap<String, PartialTaskConfig>> {
        for (name, script) in scripts {
            if self.is_valid(name, script) {
                self.tasks
                    .insert(self.create_task_id(name)?, self.create_task(name, script)?);
            }
        }

        Ok(self.tasks)
    }

    fn create_task_id(&self, name: &str) -> AnyResult<String> {
        Ok(Id::clean(name)?.as_str().trim_matches('-').to_string())
    }

    fn create_task(&self, name: &str, script: &str) -> AnyResult<PartialTaskConfig> {
        let package_manager = self.config.package_manager.unwrap_or_default();
        let script_args = shell_words::split(script)?;

        let mut config = PartialTaskConfig::default();
        let mut options = PartialTaskOptionsConfig::default();
        let mut modify_options = false;

        match package_manager {
            JavaScriptPackageManager::Deno => {
                config.description = Some(format!("Inherited from `{name}` deno.json task."));

                // command + args
                config.command = Some(PartialTaskArgs::List(vec![
                    "deno".to_string(),
                    "task".to_string(),
                    name.to_string(),
                ]));
            }
            _ => {
                config.description = Some(format!("Inherited from `{name}` package.json script."));

                // command + args
                config.command = Some(PartialTaskArgs::List(vec![
                    package_manager.to_string(),
                    "run".to_string(),
                    name.to_string(),
                ]));
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
                config
                    .outputs
                    .get_or_insert_default()
                    .push(OutputPath::ProjectFile(output_path));
            }
        }

        // preset + local
        #[allow(deprecated)]
        if self.is_dev_script_name(name) {
            config.local = Some(true);
            config.preset = Some(if self.has_watch_option(script) {
                TaskPreset::Watcher
            } else {
                TaskPreset::Server
            });
        }

        // toolchains
        if matches!(
            package_manager,
            JavaScriptPackageManager::Bun | JavaScriptPackageManager::Deno
        ) {
            config.toolchain = Some(OneOrMany::Many(vec![
                Id::raw("javascript"),
                package_manager.get_runtime_toolchain(),
            ]));
        } else {
            config.toolchain = Some(OneOrMany::Many(vec![
                Id::raw("javascript"),
                Id::raw(package_manager.to_string()),
                package_manager.get_runtime_toolchain(),
            ]));
        }

        // options
        if self.has_help_or_version_option(script) || self.has_watch_option(script) {
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

    fn has_help_or_version_option(&self, script: &str) -> bool {
        for option in ["--help", "--version"] {
            if script.contains(option) {
                return true;
            }
        }

        false
    }

    fn has_watch_option(&self, script: &str) -> bool {
        script.contains("--watch") || script.contains("-w")
    }

    fn is_valid(&self, name: &str, script: &str) -> bool {
        if self.life_cycles.contains(name)
            || name.starts_with("pre")
            || name.starts_with("post")
            || script.is_empty()
        {
            return false;
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
