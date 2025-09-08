// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::DenoToolchainConfig;
use crate::deno_json::DenoJson;
use extism_pdk::*;
use moon_config::BinEntry;
use moon_pdk::{
    get_host_env_var, get_host_environment, parse_toolchain_config, parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use std::path::PathBuf;

fn gather_shared_paths(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(globals_dir) = globals_dir
        && globals_dir.real_path().is_some()
    {
        // Avoid the host env overhead if we already
        // have a valid globals directory!
        return Ok(());
    }

    if let Some(dir) = var::get::<String>("bin_dir")? {
        paths.push(PathBuf::from(dir));
    } else {
        let maybe_dir = if let Some(value) = get_host_env_var("DENO_INSTALL_ROOT")? {
            Some(PathBuf::from(value).join("bin"))
        } else if let Some(value) = get_host_env_var("DENO_HOME")? {
            Some(PathBuf::from(value).join("bin"))
        } else {
            env.home_dir.join(".deno").join("bin").real_path()
        };

        if let Some(dir) = maybe_dir {
            if let Some(dir_str) = dir.to_str() {
                var::set("bin_dir", dir_str)?;
            }

            paths.push(dir);
        }
    }

    Ok(())
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let config = parse_toolchain_config::<DenoToolchainConfig>(input.toolchain_config)?;
    let env = get_host_environment()?;

    if input.command == "deno" && !config.execute_args.is_empty() {
        output.args = Some(Extend::Prepend(config.execute_args));
    }

    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;

    gather_shared_paths(&env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn parse_manifest(
    Json(input): Json<ParseManifestInput>,
) -> FnResult<Json<ParseManifestOutput>> {
    let mut output = ParseManifestOutput::default();
    let manifest = DenoJson::load(input.path)?;

    // https://docs.deno.com/runtime/fundamentals/configuration/#dependencies
    // https://docs.deno.com/runtime/fundamentals/modules/
    for (name, specifier) in &manifest.imports {
        if name == "." || name == "./" || name == "/" {
            continue;
        }

        let mut config = ManifestDependencyConfig::default();

        if specifier.starts_with("http") {
            config.url = Some(specifier.to_owned());
        } else if specifier.starts_with(".") || specifier.starts_with("/") {
            config.path = Some(specifier.into());
        } else if specifier.starts_with("npm:") || specifier.starts_with("jsr:") {
            if let Some(index) = specifier.rfind('@') {
                config.version = Some(UnresolvedVersionSpec::parse(&specifier[index + 1..])?);
            }

            // TODO track specifier
        }

        output
            .dependencies
            .insert(name.to_owned(), ManifestDependency::Config(config));
    }

    if let Some(version) = &manifest.version {
        output.version = Some(Version::parse(version)?);
    }

    // https://docs.deno.com/runtime/reference/cli/publish/#package-requirements
    output.publishable =
        manifest.name.is_some() && manifest.version.is_some() && manifest.exports.is_some();

    Ok(Json(output))
}

#[plugin_fn]
pub fn setup_environment(
    Json(input): Json<SetupEnvironmentInput>,
) -> FnResult<Json<SetupEnvironmentOutput>> {
    let config = parse_toolchain_config_schema::<DenoToolchainConfig>(input.toolchain_config)?;
    let mut output = SetupEnvironmentOutput::default();

    // Install binaries
    // https://docs.deno.com/runtime/reference/cli/install/
    if !config.bins.is_empty() {
        let env = get_host_environment()?;

        for bin in &config.bins {
            let mut args = vec!["install", "--global", "--allow-net", "--allow-read"];

            let name = match bin {
                BinEntry::Name(bin) => {
                    args.push(bin);
                    bin
                }
                BinEntry::Config(cfg) => {
                    if cfg.local && env.ci {
                        continue;
                    }

                    if let Some(name) = &cfg.name {
                        args.push("--name");
                        args.push(name);
                    }

                    if cfg.force {
                        args.push("--force");
                    }

                    args.push(&cfg.bin);
                    &cfg.bin
                }
            };

            output.commands.push(
                ExecCommand::new(ExecCommandInput::new("deno", args).cwd(input.root.to_owned()))
                    .cache(format!("deno-bin-{name}")),
            );
        }
    }

    Ok(Json(output))
}
