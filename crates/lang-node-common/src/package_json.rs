use nodejs_package_json::PackageJson;
use proto_pdk_api::{AnyResult, VirtualPath};
use serde_json::Value;
use std::fs;

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

pub fn extract_engine_version<'a>(package_json: &'a PackageJson, key: &str) -> Option<&'a String> {
    if let Some(engines) = &package_json.engines {
        return engines.get(key);
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
        if let Some(Value::String(inner)) = volta.get(key) {
            return Ok(Some(inner.into()));
        }

        if let Some(Value::String(extends_from)) = volta.get("extends") {
            let extends_path = package_path.parent().unwrap().join(extends_from);

            if extends_path.exists() && extends_path.is_file() {
                let content = fs::read_to_string(&extends_path)?;

                if let Ok(package_json) = serde_json::from_str::<PackageJson>(&content) {
                    return extract_volta_version(&package_json, &extends_path, key);
                }
            }
        }
    }

    Ok(None)
}
