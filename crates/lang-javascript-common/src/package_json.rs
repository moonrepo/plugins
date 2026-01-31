use nodejs_package_json::{DevEngineField, OneOrMany, PackageJson, VersionProtocol};
use proto_pdk_api::{AnyResult, VirtualPath};
use starbase_utils::{fs, json};

pub fn extract_valid_version_protocol(version_protocol: &VersionProtocol) -> Option<String> {
    if matches!(
        version_protocol,
        VersionProtocol::Range(_) | VersionProtocol::Requirement(_) | VersionProtocol::Version(_)
    ) {
        Some(version_protocol.to_string())
    } else {
        None
    }
}

pub fn extract_version_from_text(content: &str) -> Option<&str> {
    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        } else {
            return Some(line);
        }
    }

    None
}

pub fn extract_dev_engine_runtime_version(package_json: &PackageJson, key: &str) -> Option<String> {
    if let Some(engines) = &package_json.dev_engines
        && let Some(engine) = &engines.runtime
    {
        for item in engine.list() {
            if item.name == key
                && let Some(protocol) = &item.version
                && let Some(version) = extract_valid_version_protocol(protocol)
            {
                return Some(version);
            }
        }
    }

    None
}

pub fn extract_dev_engine_package_manager_version(
    package_json: &PackageJson,
    key: &str,
) -> Option<String> {
    if let Some(engines) = &package_json.dev_engines
        && let Some(engine) = &engines.package_manager
    {
        for item in engine.list() {
            if item.name == key
                && let Some(protocol) = &item.version
                && let Some(version) = extract_valid_version_protocol(protocol)
            {
                return Some(version);
            }
        }
    }

    None
}

pub fn extract_engine_version(package_json: &PackageJson, key: &str) -> Option<String> {
    if let Some(engines) = &package_json.engines {
        return engines.get(key).and_then(extract_valid_version_protocol);
    }

    None
}

pub fn extract_package_manager_version<'a>(
    package_json: &'a PackageJson,
    key: &str,
) -> Option<&'a str> {
    if let Some(pm) = &package_json.package_manager {
        let mut parts = pm.split('@');
        let name = parts.next().unwrap_or_default();

        if name == key {
            let value = if let Some(value) = parts.next() {
                // Remove corepack build metadata hash
                if let Some(index) = value.find('+') {
                    &value[0..index]
                } else {
                    value
                }
            } else {
                "latest"
            };

            return Some(value);
        }
    }

    None
}

pub fn extract_volta_version(
    package_json: &PackageJson,
    package_path: &VirtualPath,
    key: &str,
) -> AnyResult<Option<String>> {
    if let Some(volta) = package_json.other_fields.get("volta") {
        if let Some(json::JsonValue::String(inner)) = volta.get(key) {
            return Ok(Some(inner.into()));
        }

        if let Some(json::JsonValue::String(extends_from)) = volta.get("extends") {
            let extends_path = package_path.parent().unwrap().join(extends_from);

            if extends_path.exists() && extends_path.is_file() {
                let content = fs::read_file(&extends_path)?;

                if let Ok(other_package_json) = json::parse::<PackageJson>(&content) {
                    return extract_volta_version(&other_package_json, &extends_path, key);
                }
            }
        }
    }

    Ok(None)
}

pub fn insert_dev_engine_version(
    package_json: &mut PackageJson,
    kind: String,
    name: String,
    version: String,
) -> AnyResult<()> {
    let dev_engines = package_json.dev_engines.get_or_insert_default();

    let dev_engine = if kind == "runtime" {
        dev_engines
            .runtime
            .get_or_insert_with(|| OneOrMany::One(DevEngineField::default()))
    } else {
        dev_engines
            .package_manager
            .get_or_insert_with(|| OneOrMany::One(DevEngineField::default()))
    };

    match dev_engine {
        OneOrMany::One(engine) => {
            engine.name = name.into();
            engine.version = Some(VersionProtocol::try_from(version)?);
        }
        OneOrMany::Many(engines) => {
            if let Some(existing) = engines.iter_mut().find(|engine| engine.name == name) {
                existing.name = name.into();
                existing.version = Some(VersionProtocol::try_from(version)?);
            } else {
                engines.push(DevEngineField {
                    name: name.into(),
                    version: Some(VersionProtocol::try_from(version)?),
                    ..Default::default()
                });
            }
        }
    };

    Ok(())
}

pub fn remove_dev_engine(
    package_json: &mut PackageJson,
    kind: String,
    name: String,
) -> AnyResult<Option<String>> {
    let mut removed_version = None;

    if let Some(dev_engines) = &mut package_json.dev_engines {
        let mut remove_kind = false;

        let dev_engine = if kind == "runtime" {
            dev_engines.runtime.as_mut()
        } else {
            dev_engines.package_manager.as_mut()
        };

        if let Some(dev_engine) = dev_engine {
            match dev_engine {
                OneOrMany::One(engine) => {
                    if engine.name == name {
                        remove_kind = true;
                        removed_version =
                            engine.version.as_ref().map(|version| version.to_string());
                    }
                }
                OneOrMany::Many(engines) => {
                    engines.retain(|engine| {
                        if engine.name == name {
                            removed_version =
                                engine.version.as_ref().map(|version| version.to_string());
                            false
                        } else {
                            true
                        }
                    });
                    remove_kind = engines.is_empty();
                }
            };
        }

        if remove_kind {
            if kind == "runtime" {
                dev_engines.runtime = None;
            } else {
                dev_engines.package_manager = None;
            }
        }
    }

    Ok(removed_version)
}
