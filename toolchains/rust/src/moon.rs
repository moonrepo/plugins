use crate::cargo_toml::CargoToml;
use crate::config::RustToolchainConfig;
use cargo_toml::DepsSet;
use extism_pdk::*;
use moon_config::DependencyScope;
use moon_pdk_api::*;
use schematic::SchemaBuilder;

#[plugin_fn]
pub fn extend_project_graph(
    Json(input): Json<ExtendProjectGraphInput>,
) -> FnResult<Json<ExtendProjectGraphOutput>> {
    let mut output = ExtendProjectGraphOutput::default();

    for (id, source) in input.project_sources {
        let cargo_toml_path = input.context.workspace_root.join(source).join("Cargo.toml");
        let mut project_output = ExtendProjectOutput::default();

        let mut extract_implicit_deps =
            |package_deps: &DepsSet, scope: DependencyScope| -> AnyResult<()> {
                for (dep_name, dep) in package_deps {
                    // Only inherit if the dependency is using the local `path = "..."` syntax
                    if dep.detail().is_some_and(|det| det.path.is_some()) {
                        project_output.dependencies.push(ProjectDependency {
                            id: dep_name.into(),
                            scope,
                        });
                    }
                }

                Ok(())
            };

        if cargo_toml_path.exists() {
            let cargo = CargoToml::load(cargo_toml_path.clone())?;

            if let Some(package) = &cargo.package {
                output.input_files.push(cargo_toml_path);
                project_output.alias = Some(package.name.clone());

                extract_implicit_deps(&cargo.dependencies, DependencyScope::Production)?;
                extract_implicit_deps(&cargo.dev_dependencies, DependencyScope::Development)?;
                extract_implicit_deps(&cargo.build_dependencies, DependencyScope::Build)?;

                output.extended_projects.insert(id, project_output);
            }
        }
    }

    Ok(Json(output))
}
