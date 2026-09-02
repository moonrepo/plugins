use crate::config::DotnetToolConfig;
use crate::global_json;
use crate::metadata::{ChannelReleases, ChannelSummary, ReleasesIndex, channel_of};
use crate::rid::target_rid;
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::HashMap;
use tool_common::enable_tracing;

#[host_fn]
extern "ExtismHost" {
    fn host_log(input: Json<HostLogInput>);
}

static NAME: &str = ".NET";

fn fetch_index(metadata_url: &str) -> AnyResult<ReleasesIndex> {
    fetch_json(format!("{metadata_url}/releases-index.json"))
}

fn fetch_channel(metadata_url: &str, channel: &str) -> AnyResult<ChannelReleases> {
    fetch_json(format!("{metadata_url}/{channel}/releases.json"))
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    enable_tracing();

    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        // 0.60, not 0.61: moon 2.5 embeds proto_core 0.60.4, and this plugin
        // is loaded as a moon toolchain as well as a proto tool. Declaring 0.61
        // (as the proto-only swift tool does) would make moon refuse it
        // outright. Nothing here needs a 0.61 API.
        minimum_proto_version: Some(Version::new(0, 60, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<DotnetToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec!["global.json".into()],
        ignore: vec!["bin".into(), "obj".into()],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    // A `global.json` that fails to parse is the SDK's problem to report, and
    // it is not necessarily about the SDK version at all — the file also
    // carries `msbuild-sdks` and `projects`. Yield no version rather than
    // failing version detection outright.
    let version = global_json::parse(&input.content).unwrap_or_default();

    Ok(Json(ParseVersionFileOutput { version }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let config = get_tool_config::<DotnetToolConfig>()?;
    let index = fetch_index(&config.metadata_url)?;

    // One request per channel, EOL ones included, because an old SDK is still a
    // legitimate thing to pin — and not only as an exact version. The
    // `global.json` mapping turns `rollForward` into a *range*, so a legacy
    // repository pinning 3.1 or 6.0 needs those channels present here to
    // resolve at all. Walking only the supported channels would cost 5 requests
    // instead of 15, but it would break exactly the repositories most likely to
    // be on an old SDK.
    //
    // The cost is bounded elsewhere instead: `resolve_version` answers exact
    // versions and aliases without ever reaching this function, and proto
    // caches what this returns, so the full sweep is paid once per cache expiry
    // by range pins alone. Narrowing the set by `input.initial` is not an
    // option either, since proto caches one list per tool and a spec-dependent
    // subset would be served to later, unrelated resolutions.
    //
    // A channel that cannot be fetched is skipped rather than fatal, so one
    // missing `releases.json` cannot take out the whole listing.
    let mut versions = vec![];
    let mut unavailable = vec![];

    for channel in &index.channels {
        match fetch_channel(&config.metadata_url, &channel.channel_version) {
            Ok(releases) => {
                for sdk in releases.sdks() {
                    versions.push(sdk.version.clone());
                }
            }
            Err(_) => unavailable.push(channel.channel_version.clone()),
        }
    }

    if !unavailable.is_empty() {
        host_log!(
            warn,
            "Unable to read .NET release metadata for the {} channel(s). SDKs from those channels cannot be installed.",
            unavailable.join(", ")
        );
    }

    // Every channel failing means the metadata host is unreachable or has
    // moved, which is worth an error rather than an empty version list.
    if versions.is_empty() {
        return Err(plugin_err!(
            "Unable to load any .NET SDK versions from the release metadata at <url>{}</url>.",
            config.metadata_url
        ));
    }

    let mut output = LoadVersionsOutput::from(versions)?;

    // `lts` and `sts` are .NET's own release-cadence names, and neither proto
    // nor `LoadVersionsOutput::from` can infer them.
    for (alias, wanted) in [("lts", "lts"), ("sts", "sts")] {
        if let Some(channel) = index
            .channels
            .iter()
            .filter(|channel| channel.is_supported() && channel.release_type == wanted)
            .max_by(|a, b| compare_channels(a, b))
            && let Ok(spec) = UnresolvedVersionSpec::parse(&channel.latest_sdk)
        {
            output.aliases.insert(alias.into(), spec);
        }
    }

    if let Some(channel) = index.channels.iter().find(|channel| channel.is_preview())
        && let Ok(spec) = UnresolvedVersionSpec::parse(&channel.latest_sdk)
    {
        output.aliases.insert("preview".into(), spec);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    // Answering here is what keeps `load_versions` from running: proto only
    // falls through to the version listing when this returns no version, and
    // that listing costs one request per channel.
    //
    // Only aliases are answered. A fully qualified version could be returned
    // as-is too, and proto does exactly that when it short-circuits on its own,
    // but it deliberately does not short-circuit for `proto install` — that is
    // where it wants the version validated against the real list before
    // downloading anything. Returning one here would override that decision to
    // save requests on a command people run rarely.
    match &input.initial {
        // Every alias this plugin publishes describes a channel, and the index
        // already carries each channel's `latest-sdk`, so one request answers
        // it instead of fourteen. The answer comes from the metadata rather
        // than being asserted, so it needs no further validation. An unknown
        // alias falls through to the listing, which fails properly.
        UnresolvedVersionSpec::Alias(alias) => {
            let config = get_tool_config::<DotnetToolConfig>()?;

            if let Ok(index) = fetch_index(&config.metadata_url)
                && let Some(latest) = index.latest_sdk_for_alias(alias)
                && let Ok(version) = Version::parse(latest)
            {
                output.version = Some(VersionSpec::Version(version));
            }
        }

        // A requirement has to be matched against the real list, so it falls
        // through. Warn about the one that cannot say what it means: SDK
        // feature bands are the leading digit of the patch, and they are
        // parallel product lines rather than a sequence, so a bare channel such
        // as `8.0` resolves to the highest band (4xx today) when the repository
        // may want 1xx — a sideways move, not a newer release.
        //
        // A band cannot be written as a single requirement — `8.0.1xx` is not a
        // version — but it is expressible as a compound one, which is what the
        // `global.json` mapping emits, so the warning can point somewhere
        // concrete.
        UnresolvedVersionSpec::Requirement(req) => {
            if req.major.is_some() && req.minor.is_some() && req.patch.is_none() {
                host_log!(
                    warn,
                    "The .NET {} channel may span several SDK feature bands, and the highest is selected. Pin an exact SDK version, or a band such as <symbol>>=8.0.100 && <8.0.200</symbol>, if a specific band is required.",
                    input.initial
                );
            }
        }

        _ => {}
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;
    let version = &input.context.version;

    if version.is_canary() {
        return Err(plugin_err!(PluginError::UnsupportedCanary {
            tool: NAME.into()
        }));
    }

    let (rid, extension) = target_rid(env)?;
    let config = get_tool_config::<DotnetToolConfig>()?;
    let version_str = version.to_string();

    // An explicit `dist-url` takes over completely; its archives are not the
    // ones the metadata has hashes for.
    if let Some(dist_url) = &config.dist_url {
        let download_url = dist_url
            .replace("{version}", &version_str)
            .replace("{rid}", &rid)
            .replace("{extension}", extension);

        return Ok(Json(DownloadPrebuiltOutput {
            download_name: Some(format!("dotnet-sdk-{version_str}-{rid}.{extension}")),
            download_url,
            ..Default::default()
        }));
    }

    // The metadata carries both the archive URL and its SHA512, so one lookup
    // covers the download and its verification. Only the version's own channel
    // is fetched, not the whole index.
    let channel = channel_of(&version_str).ok_or_else(|| {
        plugin_err!(
            "Unable to derive a .NET release channel from <version>{version_str}</version>."
        )
    })?;

    let releases = fetch_channel(&config.metadata_url, &channel)?;

    let sdk = releases.find_sdk(&version_str).ok_or_else(|| {
        plugin_err!(
            "SDK <version>{version_str}</version> is not listed in the .NET {channel} release metadata.",
        )
    })?;

    let file = sdk.find_archive(&rid, extension).ok_or_else(|| {
        plugin_err!(
            "SDK <version>{version_str}</version> has no <symbol>{rid}</symbol> archive. .NET does not publish that platform for this release.",
        )
    })?;

    // The metadata always carries a hash; an empty one means the metadata has
    // been mirrored or rewritten incompletely. Installing unverified is still
    // better than refusing to install, but it must not happen quietly — this is
    // the only integrity check in the whole flow.
    if file.hash.is_empty() {
        host_log!(
            warn,
            "The .NET release metadata lists no checksum for <file>{}</file>. It will be installed without verification.",
            if file.name.is_empty() {
                version_str.as_str()
            } else {
                file.name.as_str()
            }
        );
    }

    Ok(Json(DownloadPrebuiltOutput {
        // The archive unpacks straight to the SDK root — muxer, host/fxr,
        // shared runtimes, sdk band, packs and templates — with no wrapping
        // directory to strip. Verified across old, musl and preview archives.
        archive_prefix: None,
        checksum: if file.hash.is_empty() {
            None
        } else {
            Some(Checksum::sha512(file.hash.clone()))
        },
        download_name: Some(if file.name.is_empty() {
            format!("dotnet-sdk-{version_str}-{rid}.{extension}")
        } else {
            file.name.clone()
        }),
        download_url: file.url.clone(),
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "dotnet".into(),
            ExecutableConfig {
                exe_path: Some(env.os.get_exe_name("dotnet").into()),
                primary: true,
                // The muxer locates `host/fxr` and `shared/` relative to its
                // own path on disk, so it only works from inside its install
                // directory. proto's `bin` entries are symlinks, and a
                // symlinked muxer fails with "host/fxr does not exist" —
                // shims are fine, because they execute the real path.
                no_bin: true,
                ..Default::default()
            },
        )]),
        exes_dirs: vec![".".into()],
        ..Default::default()
    }))
}

#[plugin_fn]
pub fn pin_version(Json(input): Json<PinVersionInput>) -> FnResult<Json<PinVersionOutput>> {
    let mut output = PinVersionOutput::default();
    let file = input.dir.join("global.json");

    // Consistent with the other tools that pin into an existing manifest: the
    // file is edited, never created. `global.json` also carries `msbuild-sdks`
    // and `projects`, so conjuring one here would be writing a config the
    // repository never asked for.
    if !file.exists() {
        output.error = Some("No <file>global.json</file> exists in the target directory.".into());

        return Ok(Json(output));
    }

    let mut global: serde_json::Value = starbase_utils::json::read_file(&file)?;

    let Some(root) = global.as_object_mut() else {
        output.error = Some("<file>global.json</file> is not a JSON object.".into());

        return Ok(Json(output));
    };

    let sdk = root
        .entry("sdk")
        .or_insert_with(|| serde_json::Value::Object(Default::default()));

    let Some(sdk) = sdk.as_object_mut() else {
        output.error = Some(
            "The <property>sdk</property> entry in <file>global.json</file> is not a JSON object."
                .into(),
        );

        return Ok(Json(output));
    };

    sdk.insert(
        "version".into(),
        serde_json::Value::String(input.version.to_string()),
    );

    starbase_utils::json::write_file_with_config(&file, &global, true)?;

    output.pinned = true;
    output.file = Some(file);

    Ok(Json(output))
}

#[plugin_fn]
pub fn unpin_version(Json(input): Json<UnpinVersionInput>) -> FnResult<Json<UnpinVersionOutput>> {
    let mut output = UnpinVersionOutput::default();
    let file = input.dir.join("global.json");

    if !file.exists() {
        output.error = Some("No <file>global.json</file> exists in the target directory.".into());

        return Ok(Json(output));
    }

    let mut global: serde_json::Value = starbase_utils::json::read_file(&file)?;

    let Some(root) = global.as_object_mut() else {
        output.error = Some("<file>global.json</file> is not a JSON object.".into());

        return Ok(Json(output));
    };

    let removed = root
        .get_mut("sdk")
        .and_then(|sdk| sdk.as_object_mut())
        .and_then(|sdk| sdk.remove("version"));

    if let Some(serde_json::Value::String(version)) = removed {
        // An `sdk` left holding nothing is noise, but one that still carries
        // `rollForward` or `allowPrerelease` is not ours to discard.
        if root
            .get("sdk")
            .and_then(|sdk| sdk.as_object())
            .is_some_and(|sdk| sdk.is_empty())
        {
            root.remove("sdk");
        }

        starbase_utils::json::write_file_with_config(&file, &global, true)?;

        output.unpinned = true;
        output.version = UnresolvedVersionSpec::parse(&version).ok();
        output.file = Some(file);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn activate_environment(
    Json(input): Json<ActivateEnvironmentInput>,
) -> FnResult<Json<ActivateEnvironmentOutput>> {
    let mut output = ActivateEnvironmentOutput::default();

    // For `proto activate` only — moon never calls this function (nothing in
    // moon 2.5 references it), and injects DOTNET_ROOT through the toolchain's
    // own `extend_task_command` instead.
    //
    // The install directory is a complete SDK root, and the muxer resolves its
    // own `host/fxr` and `shared/` from its path on disk, so it does not need
    // this itself. Everything else does: MSBuild, the SDK resolvers, and any
    // `dotnet` reached other than through this install read DOTNET_ROOT.
    if let Some(root) = input.context.tool_dir.to_real_path()? {
        output.env.insert("DOTNET_ROOT".into(), root.to_string());
    }

    Ok(Json(output))
}

fn compare_channels(a: &ChannelSummary, b: &ChannelSummary) -> std::cmp::Ordering {
    let parse = |channel: &ChannelSummary| {
        let mut parts = channel.channel_version.split('.');
        let major = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(0);
        let minor = parts
            .next()
            .and_then(|p| p.parse::<u32>().ok())
            .unwrap_or(0);

        (major, minor)
    };

    parse(a).cmp(&parse(b))
}
