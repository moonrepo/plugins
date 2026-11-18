// Note: Most tier 2 is implemented in the JavaScript toolchain!

use crate::config::DenoToolchainConfig;
use crate::deno_json::DenoJson;
use extism_pdk::*;
use moon_config::BinEntry;
use moon_pdk::{
    VirtualPathExt, get_host_env_var, get_host_environment, parse_toolchain_config,
    parse_toolchain_config_schema,
};
use moon_pdk_api::*;
use std::path::PathBuf;
use toolchain_common::{append_bin_dir, debug};

// `deno install --global` writes to `$DENO_INSTALL_ROOT/bin`, otherwise `~/.deno/bin`.
// moon only adds globals directories when `version` is set, and proto's globals
// directory falls back to `~/.deno/bin` even when `DENO_INSTALL_ROOT` is set
fn gather_shared_paths(
    env: &HostEnvironment,
    globals_dir: Option<&VirtualPath>,
    paths: &mut Vec<PathBuf>,
) -> AnyResult<()> {
    if let Some(dir) = var::get::<String>("bin_dir")? {
        paths.push(PathBuf::from(dir));

        return Ok(());
    }

    let dir = if let Some(value) = get_host_env_var("DENO_INSTALL_ROOT")? {
        Some(append_bin_dir(value))
    } else if let Some(dir) = globals_dir
        .map(|dir| dir.to_real_path())
        .transpose()?
        .flatten()
    {
        Some(dir.to_path_buf())
    } else {
        env.home_dir
            .join(".deno")
            .join("bin")
            .to_real_path()?
            .map(|path| path.to_path_buf())
    };

    if let Some(dir) = dir {
        if let Some(dir_str) = dir.to_str()
            && let Err(error) = var::set("bin_dir", dir_str)
        {
            debug!("Failed to cache the Deno bin directory: {error}");
        }

        paths.push(dir);
    }

    Ok(())
}

// Subcommands that take runtime flags, like `--allow-env`, after them.
// Others, like `task` or `fmt`, reject them
const RUNTIME_SUBCOMMANDS: [&str; 8] = [
    "bench", "compile", "eval", "repl", "run", "serve", "test", "x",
];

fn extend_args(args: &[String], execute_args: Vec<String>) -> Option<Extend<Vec<String>>> {
    // Skip global flags, like `-q`
    let Some(index) = args.iter().position(|arg| !arg.starts_with('-')) else {
        return Some(Extend::Prepend(execute_args));
    };
    let first = args[index].as_str();

    if RUNTIME_SUBCOMMANDS.contains(&first) {
        let mut args = args.to_vec();
        args.splice(index + 1..index + 1, execute_args);

        return Some(Extend::Replace(args));
    }

    // A script, like `deno main.ts`, takes them before it
    if first.contains(['.', '/', ':']) {
        return Some(Extend::Prepend(execute_args));
    }

    None
}

#[plugin_fn]
pub fn extend_task_command(
    Json(input): Json<ExtendTaskCommandInput>,
) -> FnResult<Json<ExtendTaskCommandOutput>> {
    let mut output = ExtendTaskCommandOutput::default();
    let config = parse_toolchain_config::<DenoToolchainConfig>(input.toolchain_config)?;
    let env = get_host_environment()?;

    if input.command == "deno" && !config.execute_args.is_empty() {
        output.args = extend_args(&input.args, config.execute_args);
    }

    gather_shared_paths(env, input.globals_dir.as_ref(), &mut output.paths)?;

    Ok(Json(output))
}

#[plugin_fn]
pub fn extend_task_script(
    Json(input): Json<ExtendTaskScriptInput>,
) -> FnResult<Json<ExtendTaskScriptOutput>> {
    let mut output = ExtendTaskScriptOutput::default();
    let env = get_host_environment()?;

    gather_shared_paths(env, input.globals_dir.as_ref(), &mut output.paths)?;

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
            if let Some(index) = specifier.rfind('@')
                && index != 4
            {
                // ADR-0008.5
                config.version = UnresolvedVersionSpec::parse(&specifier[index + 1..])
                    .inspect_err(|error| {
                        debug!("Skipping the version of {name} in deno.json: {error}")
                    })
                    .ok();
                config.reference = Some(specifier[0..index].into());
            } else {
                config.reference = Some(specifier.into());
            }
        } else if specifier.starts_with("catalog:") {
            config.reference = Some(specifier.into());
        }

        output
            .dependencies
            .insert(name.to_owned(), ManifestDependency::Config(config));
    }

    // ADR-0008.5
    if let Some(version) = &manifest.version {
        output.version = Version::parse(version)
            .inspect_err(|error| debug!("Skipping the version in deno.json: {error}"))
            .ok();
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
            // Deno refuses to overwrite an existing install, and moon
            // re-runs every command when the list changes
            let mut args = vec![
                "install",
                "--global",
                "--force",
                "--allow-net",
                "--allow-read",
            ];

            let name = match bin {
                BinEntry::String(bin) => {
                    args.push(bin);
                    bin
                }
                BinEntry::Object(cfg) => {
                    if cfg.local && env.ci {
                        continue;
                    }

                    if let Some(name) = &cfg.name {
                        args.push("--name");
                        args.push(name);
                    }

                    // Must be before the module, otherwise they're script args
                    args.extend(cfg.args.iter().map(|arg| arg.as_str()));
                    args.push(&cfg.bin);
                    &cfg.bin
                }
            };

            output.commands.push(
                ExecCommand::new(ExecCommandInput::new("deno", args).cwd(input.root.to_owned()))
                    .cache(CacheStrategy::Memory)
                    .label(format!("deno-bin-{name}")),
            );
        }
    }

    Ok(Json(output))
}
