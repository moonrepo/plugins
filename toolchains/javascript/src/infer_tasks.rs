use crate::{config::JavaScriptToolchainConfig, package_json::PackageJson};
use moon_common::Id;
use moon_config::{
    OneOrMany, OutputPath, PartialTaskArgs, PartialTaskConfig, PartialTaskOptionsConfig,
    TaskOptionRunInCI, TaskPreset,
};
use moon_pdk::AnyResult;
use std::collections::{BTreeMap, HashSet};

pub struct TasksInferrer<'a> {
    config: &'a JavaScriptToolchainConfig,
    package: &'a PackageJson,
    tasks: BTreeMap<String, PartialTaskConfig>,
    life_cycles: HashSet<String>,
}

impl<'a> TasksInferrer<'a> {
    pub fn new(
        config: &'a JavaScriptToolchainConfig,
        package: &'a PackageJson,
    ) -> TasksInferrer<'a> {
        Self {
            config,
            package,
            tasks: BTreeMap::default(),
            life_cycles: "preprepare|prepare|postprepare|prepublish|prepublishOnly|publish|postpublish|prepack|pack|postpack|preinstall|install|postinstall|preversion|version|postversion|dependencies"
                .split('|')
                .map(|lc| lc.to_string())
                .collect::<HashSet<_>>(),
        }
    }

    pub fn infer(mut self) -> AnyResult<BTreeMap<String, PartialTaskConfig>> {
        if let Some(scripts) = &self.package.scripts {
            for (name, script) in scripts {
                if self.life_cycles.contains(name)
                    || name.starts_with("pre")
                    || name.starts_with("post")
                {
                    continue;
                }

                self.tasks.insert(
                    self.create_task_id(name)?,
                    self.create_task_from_script(name, script)?,
                );
            }
        }

        Ok(self.tasks)
    }

    fn create_task_id(&self, name: &str) -> AnyResult<String> {
        Ok(Id::clean(name)?.as_str().trim_matches('-').to_string())
    }

    fn create_task_from_script(&self, name: &str, script: &str) -> AnyResult<PartialTaskConfig> {
        let package_manager = self.config.package_manager.unwrap_or_default();
        let script_args = shell_words::split(script)?;

        let mut config = PartialTaskConfig {
            description: Some(format!("Inherited from `{name}` package.json script.")),
            ..Default::default()
        };
        let mut options = PartialTaskOptionsConfig::default();
        let mut modify_options = false;

        // command + args
        config.command = Some(PartialTaskArgs::List(vec![
            package_manager.to_string(),
            "run".to_string(),
            name.to_string(),
        ]));

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
        config.toolchain = Some(OneOrMany::Many(vec![
            Id::raw("javascript"),
            Id::raw(package_manager.to_string()),
            package_manager.get_runtime_toolchain(),
        ]));

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
