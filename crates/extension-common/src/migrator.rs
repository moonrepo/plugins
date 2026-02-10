use moon_common::Id;
use moon_config::{
    LanguageType, OneOrMany, PartialInheritedTasksConfig, PartialProjectConfig,
    PartialProjectToolchainsConfig, PartialWorkspaceConfig,
};
use moon_pdk::{AnyResult, VirtualPath};
use rustc_hash::FxHashMap;
use starbase_utils::yaml;

pub struct Migrator {
    pub moon_config_dir: VirtualPath,
    pub project_configs: FxHashMap<VirtualPath, PartialProjectConfig>,
    pub root: VirtualPath,
    pub tasks_configs: FxHashMap<VirtualPath, PartialInheritedTasksConfig>,
    pub toolchain: Id,
    pub workspace_config: Option<PartialWorkspaceConfig>,
}

impl Migrator {
    pub fn new(workspace_root: &VirtualPath) -> AnyResult<Self> {
        Ok(Self {
            moon_config_dir: if workspace_root.join(".config/moon").exists() {
                workspace_root.join(".config/moon")
            } else {
                workspace_root.join(".moon")
            },
            project_configs: FxHashMap::default(),
            tasks_configs: FxHashMap::default(),
            toolchain: Id::raw("node"),
            workspace_config: None,
            root: workspace_root.to_owned(),
        })
    }

    pub fn detect_package_manager(&self) -> String {
        let mut package_manager = "npm";

        if self.root.join("bun.lock").exists()
            || self.root.join("bun.lockb").exists()
            || self.toolchain == "bun"
        {
            package_manager = "bun";
        } else if self.root.join("pnpm-lock.yaml").exists() {
            package_manager = "pnpm";
        } else if self.root.join("yarn.lock").exists() {
            package_manager = "yarn";
        }

        package_manager.to_owned()
    }

    pub fn load_project_config(
        &mut self,
        project_source: &str,
    ) -> AnyResult<&mut PartialProjectConfig> {
        let project_config_path = self.root.join(project_source).join("moon.yml");

        if !self.project_configs.contains_key(&project_config_path) {
            if project_config_path.exists() {
                self.project_configs.insert(
                    project_config_path.clone(),
                    yaml::read_file(&project_config_path)?,
                );
            } else {
                self.project_configs.insert(
                    project_config_path.clone(),
                    PartialProjectConfig {
                        language: Some(
                            if self
                                .root
                                .join(project_source)
                                .join("tsconfig.json")
                                .exists()
                            {
                                LanguageType::TypeScript
                            } else {
                                LanguageType::JavaScript
                            },
                        ),
                        toolchains: Some(PartialProjectToolchainsConfig {
                            default: Some(OneOrMany::One(self.toolchain.clone())),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                );
            }
        }

        Ok(self.project_configs.get_mut(&project_config_path).unwrap())
    }

    pub fn load_tasks_config(
        &mut self,
        scope: &str,
    ) -> AnyResult<&mut PartialInheritedTasksConfig> {
        let tasks_config_path = self
            .moon_config_dir
            .join("tasks")
            .join(format!("{scope}.yml"));

        if !self.tasks_configs.contains_key(&tasks_config_path) {
            self.tasks_configs.insert(
                tasks_config_path.clone(),
                if tasks_config_path.exists() {
                    yaml::read_file(&tasks_config_path)?
                } else {
                    PartialInheritedTasksConfig::default()
                },
            );
        }

        Ok(self.tasks_configs.get_mut(&tasks_config_path).unwrap())
    }

    pub fn load_tasks_platform_config(&mut self) -> AnyResult<&mut PartialInheritedTasksConfig> {
        self.load_tasks_config(self.toolchain.clone().as_str())
    }

    pub fn load_workspace_config(&mut self) -> AnyResult<&mut PartialWorkspaceConfig> {
        if self.workspace_config.is_none() {
            if self.moon_config_dir.join("workspace.yml").exists() {
                self.workspace_config =
                    Some(yaml::read_file(self.moon_config_dir.join("workspace.yml"))?);
            } else {
                self.workspace_config = Some(PartialWorkspaceConfig::default());
            }
        }

        Ok(self.workspace_config.as_mut().unwrap())
    }

    pub fn save_configs(&self) -> AnyResult<()> {
        if let Some(workspace_config) = &self.workspace_config {
            yaml::write_file_with_config(
                self.moon_config_dir.join("workspace.yml"),
                workspace_config,
            )?;
        }

        for (tasks_config_path, tasks_config) in &self.tasks_configs {
            yaml::write_file_with_config(tasks_config_path, tasks_config)?;
        }

        for (project_config_path, project_config) in &self.project_configs {
            yaml::write_file_with_config(project_config_path, project_config)?;
        }

        Ok(())
    }
}

pub fn create_id<T: AsRef<str>>(id: T) -> AnyResult<Id> {
    Ok(Id::clean(
        id.as_ref().replace(':', ".").trim_start_matches('@'),
    )?)
}
