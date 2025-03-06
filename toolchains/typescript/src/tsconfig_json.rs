#[cfg(feature = "wasm")]
use extism_pdk::*;
use moon_common::path::to_relative_virtual_string;
#[cfg(feature = "wasm")]
use moon_pdk::host_log;
use moon_pdk_api::{AnyResult, VirtualPath, anyhow, json_config};
use starbase_utils::json::{self, JsonValue};
use typescript_tsconfig_json::{
    CompilerOptions, CompilerOptionsPathsMap, CompilerPath, ProjectReference,
    TsConfigJson as BaseTsConfigJson,
};

#[cfg(feature = "wasm")]
#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<moon_pdk::HostLogInput>);
}

json_config!(TsConfigJson, BaseTsConfigJson);

impl TsConfigJson {
    pub fn load_with_extends(path: VirtualPath) -> AnyResult<BaseTsConfigJson> {
        let mut config = BaseTsConfigJson::default();

        for next in BaseTsConfigJson::resolve_extends_chain(path.any_path())? {
            config.extend(next.config);
        }

        Ok(config)
    }

    fn save_field(
        &self,
        field: &str,
        current_value: Option<&JsonValue>,
    ) -> AnyResult<Option<JsonValue>> {
        Ok(match field {
            "include" => self.include.as_ref().map(|include| {
                JsonValue::from_iter(include.iter().map(|i| i.to_string()).collect::<Vec<_>>())
            }),
            "references" => {
                if let Some(references) = &self.references {
                    let mut list = vec![];

                    for reference in references {
                        let mut item = json::json!({});
                        item["path"] = JsonValue::from(reference.path.as_str());

                        if let Some(prepend) = reference.prepend {
                            item["prepend"] = JsonValue::from(prepend);
                        }

                        list.push(item);
                    }

                    Some(JsonValue::Array(list))
                } else {
                    None
                }
            }
            "compilerOptions" => {
                if let Some(options) = &self.compiler_options {
                    let mut current = current_value.cloned().unwrap_or_default();
                    let mut save = false;

                    if !current.is_object() {
                        current = json::json!({});
                    }

                    if let Some(out_dir) = &options.out_dir {
                        save = true;
                        current["outDir"] = JsonValue::from(out_dir.as_str());
                    }

                    if let Some(paths) = &options.paths {
                        save = true;
                        current["paths"] =
                            JsonValue::from_iter(paths.iter().map(|(key, value)| {
                                (
                                    key.to_owned(),
                                    value.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
                                )
                            }));
                    }

                    if save { Some(current) } else { None }
                } else {
                    None
                }
            }
            _ => None,
        })
    }
}

impl TsConfigJson {
    /// Convert an absolute virtual path to a relative virtual string,
    /// for use within tsconfig include, exclude, and other paths.
    pub fn to_relative_path(&self, path: impl AsRef<VirtualPath>) -> AnyResult<String> {
        to_relative_virtual_string(path.as_ref().any_path(), self.path.parent().any_path())
            .map_err(|error| anyhow!("{error}"))
    }

    /// Add an include pattern to the `include` field with the defined
    /// path, and sort the list based on path.
    /// Return true if the new value is different from the old value.
    pub fn add_include(&mut self, path: &VirtualPath) -> AnyResult<bool> {
        let include_path = self.to_relative_path(path)?;
        let include = self.data.include.get_or_insert_default();

        if include.iter().any(|p| p.as_str() == include_path) {
            return Ok(false);
        }

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Adding <file>{include_path}</file> as an include to <path>{}</path>",
                self.path.display(),
            );
        }

        include.push(CompilerPath::from(include_path));
        include.sort();

        self.dirty.push("include".into());

        Ok(true)
    }

    /// Add a project reference to the `references` field with the defined
    /// path and tsconfig file name, and sort the list based on path.
    /// Return true if the new value is different from the old value.
    pub fn add_project_ref(&mut self, path: &VirtualPath, tsconfig_name: &str) -> AnyResult<bool> {
        // File name is optional when using standard naming
        let ref_path = self.to_relative_path(if tsconfig_name != "tsconfig.json" {
            path.join(tsconfig_name)
        } else {
            path.to_owned()
        })?;

        let references = self.data.references.get_or_insert_default();

        // Check if the reference already exists
        if references.iter().any(|r| r.path.as_str() == ref_path) {
            return Ok(false);
        }

        #[cfg(feature = "wasm")]
        {
            host_log!(
                "Adding <file>{ref_path}</file> as a reference to <path>{}</path>",
                self.path.display(),
            );
        }

        // Add and sort the references
        references.push(ProjectReference {
            path: CompilerPath::from(ref_path),
            prepend: None,
        });

        references.sort_by_key(|r| r.path.clone());

        self.dirty.push("references".into());

        Ok(true)
    }

    /// Update `compilerOptions` using a callback function. If no
    /// options have been defined, an empty object will be inserted.
    /// The callback must return a boolean denoting whether the options
    /// have been changed at all.
    pub fn update_compiler_options<F>(&mut self, updater: F) -> bool
    where
        F: FnOnce(&mut CompilerOptions) -> bool,
    {
        let updated;

        if let Some(options) = self.data.compiler_options.as_mut() {
            updated = updater(options);
        } else {
            let mut options = CompilerOptions::default();

            updated = updater(&mut options);

            if updated {
                self.data.compiler_options = Some(options);
            }
        }

        if updated {
            self.dirty.push("compilerOptions".into());
        }

        updated
    }

    /// Update and merge the `paths` map in `compilerOptions` with the provided paths.
    pub fn update_compiler_option_paths(&mut self, paths: CompilerOptionsPathsMap) -> bool {
        self.update_compiler_options(|options| {
            let mut updated = false;

            if let Some(current_paths) = options.paths.as_mut() {
                for (path, mut patterns) in paths {
                    if let Some(current_patterns) = current_paths.get_mut(&path) {
                        patterns.sort();
                        current_patterns.sort();

                        if &patterns != current_patterns {
                            updated = true;
                            current_paths.insert(path, patterns);
                        }
                    } else {
                        updated = true;
                        current_paths.insert(path, patterns);
                    }
                }
            } else {
                updated = true;
                options.paths = Some(paths);
            }

            updated
        })
    }
}
