use nodejs_package_json::{PackageJson, VersionProtocol};
use proto_pdk_api::{AnyResult, VirtualPath};
use starbase_utils::{
    fs,
    json::{self, JsonMap, JsonValue},
};

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
    package_json: &mut JsonValue,
    kind: String,
    name: String,
    version: String,
) -> AnyResult<()> {
    if let Some(root) = package_json.as_object_mut() {
        if root.get("devEngines").is_none_or(|node| !node.is_object()) {
            root.insert("devEngines".into(), JsonValue::Object(JsonMap::default()));
        }

        if let Some(JsonValue::Object(dev_engines)) = root.get_mut("devEngines") {
            if dev_engines.get(&kind).is_none_or(|node| !node.is_object()) {
                dev_engines.insert(kind.clone(), JsonValue::Object(JsonMap::default()));
            }

            let mut convert_to_list = false;
            let mut new_list = vec![];

            if let Some(runtime_or_pm) = dev_engines.get_mut(&kind) {
                match runtime_or_pm {
                    JsonValue::Array(list) => {
                        let mut inserted = false;

                        for item in list.iter_mut() {
                            if let Some(object) = item.as_object_mut()
                                && object
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .is_some_and(|n| n == name)
                            {
                                object.insert("version".into(), JsonValue::String(version.clone()));
                                inserted = true;
                                break;
                            }
                        }

                        if !inserted {
                            list.push(JsonValue::Object(JsonMap::from_iter([
                                ("name".to_owned(), JsonValue::String(name)),
                                ("version".to_owned(), JsonValue::String(version)),
                            ])));
                        }
                    }
                    JsonValue::Object(object) => {
                        if object.is_empty() {
                            object.insert("name".into(), JsonValue::String(name));
                            object.insert("version".into(), JsonValue::String(version));
                        } else if object
                            .get("name")
                            .and_then(|n| n.as_str())
                            .is_some_and(|n| n == name)
                        {
                            object.insert("version".into(), JsonValue::String(version));
                        } else {
                            convert_to_list = true;

                            new_list.push(JsonValue::Object(object.to_owned()));
                            new_list.push(JsonValue::Object(JsonMap::from_iter([
                                ("name".to_owned(), JsonValue::String(name)),
                                ("version".to_owned(), JsonValue::String(version)),
                            ])));
                        }
                    }
                    _ => {}
                };
            }

            if convert_to_list {
                dev_engines.insert(kind, JsonValue::Array(new_list));
            }
        }
    }

    Ok(())
}

pub fn remove_dev_engine(
    package_json: &mut JsonValue,
    kind: String,
    name: String,
) -> AnyResult<Option<String>> {
    let mut removed_version = None;

    if let Some(root) = package_json.as_object_mut()
        && let Some(JsonValue::Object(dev_engines)) = root.get_mut("devEngines")
    {
        let mut remove_kind = false;

        if let Some(runtime_or_pm) = dev_engines.get_mut(&kind) {
            match runtime_or_pm {
                JsonValue::Array(list) => {
                    list.retain_mut(|item| {
                        if let Some(object) = item.as_object_mut()
                            && object
                                .get("name")
                                .and_then(|n| n.as_str())
                                .is_some_and(|n| n == name)
                        {
                            removed_version = object.remove("version").and_then(|version| {
                                version.as_str().map(|version| version.to_owned())
                            });
                            false
                        } else {
                            true
                        }
                    });
                    remove_kind = list.is_empty();
                }
                JsonValue::Object(object) => {
                    if object
                        .get("name")
                        .and_then(|n| n.as_str())
                        .is_some_and(|n| n == name)
                    {
                        remove_kind = true;
                        removed_version = object
                            .remove("version")
                            .and_then(|version| version.as_str().map(|version| version.to_owned()));
                    }
                }
                _ => {}
            };
        }

        if remove_kind {
            dev_engines.remove(&kind);
        }
    }

    Ok(removed_version)
}
