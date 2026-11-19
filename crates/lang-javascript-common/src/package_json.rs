use nodejs_package_json::{PackageJson, VersionProtocol};
use proto_pdk_api::{AnyResult, UnresolvedVersionSpec, VirtualPath};
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

pub fn extract_package_manager_version(package_json: &PackageJson, key: &str) -> Option<String> {
    if let Some(pm) = &package_json.package_manager {
        // Only split once, as Corepack also accepts a URL, like `yarn@https://…/@yarnpkg/…`
        let mut parts = pm.splitn(2, '@');
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

            return Some(value.into());
        }
    }

    None
}

pub fn extract_volta_version(
    package_json: &PackageJson,
    package_path: &VirtualPath,
    key: &str,
) -> AnyResult<Option<String>> {
    extract_volta_version_with_depth(package_json, package_path, key, 0)
}

// `extends` can form a cycle, which would overflow the stack
const VOLTA_MAX_EXTENDS: usize = 10;

fn extract_volta_version_with_depth(
    package_json: &PackageJson,
    package_path: &VirtualPath,
    key: &str,
    depth: usize,
) -> AnyResult<Option<String>> {
    if depth > VOLTA_MAX_EXTENDS {
        return Ok(None);
    }

    if let Some(volta) = package_json.other_fields.get("volta") {
        if let Some(json::JsonValue::String(inner)) = volta.get(key) {
            return Ok(Some(inner.into()));
        }

        if let Some(json::JsonValue::String(extends_from)) = volta.get("extends") {
            let extends_path = package_path
                .parent()
                .expect("file path should have a parent")
                .join(extends_from);

            if extends_path.exists() && extends_path.is_file() {
                let content = fs::read_file(&extends_path)?;

                if let Ok(other_package_json) = json::parse::<PackageJson>(&content) {
                    return extract_volta_version_with_depth(
                        &other_package_json,
                        &extends_path,
                        key,
                        depth + 1,
                    );
                }
            }
        }
    }

    Ok(None)
}

/// `devEngines` only supports versions and ranges, so return an error
/// for aliases (like `lts`), which package managers reject.
pub fn get_dev_engine_version_error(version: &UnresolvedVersionSpec) -> Option<String> {
    matches!(
        version,
        UnresolvedVersionSpec::Alias(_) | UnresolvedVersionSpec::Canary
    )
    .then(|| {
        format!(
            "Unable to pin <version>{version}</version>, as <property>devEngines</property> in <file>package.json</file> only supports versions and ranges, not aliases."
        )
    })
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
            // A list holds one entry per tool, like Node.js and Bun
            if dev_engines
                .get(&kind)
                .is_none_or(|node| !node.is_object() && !node.is_array())
            {
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
                JsonValue::Object(object)
                    if object
                        .get("name")
                        .and_then(|n| n.as_str())
                        .is_some_and(|n| n == name) =>
                {
                    remove_kind = true;
                    removed_version = object
                        .remove("version")
                        .and_then(|version| version.as_str().map(|version| version.to_owned()));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_dev_engine_versions_into_lists() {
        let mut package_json = serde_json::json!({
            "devEngines": {
                "packageManager": [
                    { "name": "npm", "version": "^11.0.0" },
                    { "name": "pnpm", "version": "^10.0.0", "onFail": "warn" }
                ]
            }
        });

        insert_dev_engine_version(
            &mut package_json,
            "packageManager".into(),
            "pnpm".into(),
            "10.33.4".into(),
        )
        .unwrap();
        insert_dev_engine_version(
            &mut package_json,
            "packageManager".into(),
            "yarn".into(),
            "4.9.0".into(),
        )
        .unwrap();

        assert_eq!(
            package_json["devEngines"]["packageManager"],
            serde_json::json!([
                { "name": "npm", "version": "^11.0.0" },
                { "name": "pnpm", "version": "10.33.4", "onFail": "warn" },
                { "name": "yarn", "version": "4.9.0" }
            ])
        );
    }

    #[test]
    fn extracts_node_versions_when_other_fields_are_invalid() {
        let package_json: PackageJson = json::parse(
            r#"{
                "version": "1.0",
                "bundleDependencies": true,
                "dependencies": { "foo": "^v1.2.3", "bar": ">=1.0.0-" },
                "engines": { "npm": "not a version", "node": ">=22" },
                "devEngines": { "runtime": { "name": "node", "version": "^24" } },
                "volta": { "node": "22.22.3" }
            }"#,
        )
        .unwrap();

        assert_eq!(
            extract_engine_version(&package_json, "node").as_deref(),
            Some(">=22")
        );
        assert_eq!(
            extract_dev_engine_runtime_version(&package_json, "node").as_deref(),
            Some("^24")
        );
        assert_eq!(
            extract_volta_version(&package_json, &VirtualPath::new("/package.json"), "node")
                .unwrap()
                .as_deref(),
            Some("22.22.3")
        );
    }

    #[test]
    fn errors_for_dev_engine_aliases() {
        for version in ["lts", "latest", "canary"] {
            assert!(
                get_dev_engine_version_error(&UnresolvedVersionSpec::parse(version).unwrap())
                    .is_some(),
                "{version}"
            );
        }

        for version in ["20", "^20.1", ">=18 <22", "20.18.0"] {
            assert!(
                get_dev_engine_version_error(&UnresolvedVersionSpec::parse(version).unwrap())
                    .is_none(),
                "{version}"
            );
        }
    }
}
