//! On-disk cache of evaluated NuGet package sets, keyed per moon project.
//!
//! Task hashing needs the same data the project graph just evaluated, but runs
//! later — often in a separate process, against an already-cached project graph
//! — so it cannot rely on in-memory state. Without this, a workspace with no
//! lock files pays one MSBuild evaluation *per project* while hashing, which is
//! exactly what batching the graph evaluation exists to avoid.

use crate::discovery::{find_config_files, find_project_files, walk_up};
use moon_pdk_api::VirtualPath;
use serde::{Deserialize, Serialize};
use starbase_utils::fs;
use std::collections::BTreeMap;

/// FNV-1a digest, rendered hex. Used only to discriminate cache keys, never
/// for integrity — a plain content hash would mean pulling sha2 into the
/// wasm binary. Deterministic across Rust versions, unlike `DefaultHasher`.
pub fn content_digest(content: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in content.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    format!("{hash:016x}")
}

/// Cached evaluated package set for one moon project, written by the batched
/// graph evaluation and read back by task hashing.
#[derive(Debug, Deserialize, Serialize)]
struct EvalCacheEntry {
    /// Digest of every file that can change the evaluated package set, so a
    /// stale entry is never used.
    digest: String,
    packages: BTreeMap<String, String>,
}

/// Where cached package sets live. Under `.moon/cache`, which moon already
/// treats as disposable.
fn eval_cache_file(workspace_root: &VirtualPath, project_id: &str) -> VirtualPath {
    let safe_id = project_id
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.') {
                char
            } else {
                '_'
            }
        })
        .collect::<String>();

    workspace_root
        .join(".moon")
        .join("cache")
        .join("dotnet-toolchain")
        .join("eval")
        .join(format!("{safe_id}.json"))
}

/// Append one file to the digest buffer, framed by its name and byte length.
///
/// Framing is load-bearing. Concatenating contents directly means the same bytes
/// distributed differently across two files produce the same buffer: moving a
/// `<PackageVersion>` block from the end of `Directory.Build.props` to the start
/// of `Directory.Packages.props` — adjacent in `find_config_files`' name sort —
/// is a routine Central Package Management migration, and it left the digest, and
/// therefore every task hash, unchanged.
///
/// The file *name* is used rather than the full path, so the digest stays
/// independent of where the workspace lives on disk.
///
/// An unreadable file contributes an empty body rather than propagating: that
/// yields a digest that cannot match the one recorded when the file was
/// readable, i.e. a cache miss, which is the safe direction. Returning an error
/// instead would break `write_eval_cache`'s best-effort contract.
fn push_framed(buffer: &mut String, file: &VirtualPath) {
    let content = fs::read_file(file).unwrap_or_default();
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();

    buffer.push_str(&name);
    buffer.push(':');
    buffer.push_str(&content.len().to_string());
    buffer.push(':');
    buffer.push_str(&content);
}

/// Digest of everything that can change a project's evaluated package set:
/// its project files, every config file from the project directory up to the
/// workspace root, plus the configured `msbuildProperties` — a conditional
/// `PackageReference` gated on such a property evaluates differently when the
/// setting changes, so the properties must invalidate the cache like any file
/// edit would.
///
/// Two things are deliberately *not* captured. Custom `<Import>`s outside the
/// `Directory.Build.*` conventions — the same caveat that already applies to
/// task hashing itself. And the identity of the SDK that produced the set, so
/// switching `dotnetRoot` or upgrading the system SDK reuses the old answer
/// until some file changes. Including it would mean resolving the SDK root
/// before the cache *read* in `hash_task_contents`, and any asymmetry between
/// the read and write keys turns the cache into a permanent miss — which is the
/// per-project-evaluation cost this cache exists to avoid.
fn eval_cache_digest(
    project_root: &VirtualPath,
    workspace_root: &VirtualPath,
    msbuild_properties: &BTreeMap<String, String>,
) -> String {
    let mut buffer = String::new();

    for file in find_project_files(project_root) {
        push_framed(&mut buffer, &file);
    }

    for dir in walk_up(project_root, workspace_root) {
        for file in find_config_files(&dir) {
            push_framed(&mut buffer, &file);
        }
    }

    // Framed like a file, under a name no real file can have (path separator).
    for (name, value) in msbuild_properties {
        buffer.push_str("msbuild-property/");
        buffer.push_str(name);
        buffer.push(':');
        buffer.push_str(&value.len().to_string());
        buffer.push(':');
        buffer.push_str(value);
    }

    content_digest(&buffer)
}

/// Persist a project's evaluated package set for task hashing to reuse.
///
/// Callers must only pass a set they evaluated *completely*: a partial set is
/// indistinguishable from a complete one once written, and it would be served
/// under a digest that keeps validating.
pub fn write_eval_cache(
    workspace_root: &VirtualPath,
    project_id: &str,
    project_root: &VirtualPath,
    msbuild_properties: &BTreeMap<String, String>,
    packages: BTreeMap<String, String>,
) {
    let file = eval_cache_file(workspace_root, project_id);

    let entry = EvalCacheEntry {
        digest: eval_cache_digest(project_root, workspace_root, msbuild_properties),
        packages,
    };

    // Best-effort: a failed write only costs a re-evaluation later. Two tasks
    // of the same project can race here, but they write identical content and
    // a torn read simply fails to parse (also a re-evaluation).
    if let Some(parent) = file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string(&entry) {
        let _ = fs::write_file(&file, json);
    }
}

/// Read a project's cached package set, if it is still current.
pub fn read_eval_cache(
    workspace_root: &VirtualPath,
    project_id: &str,
    project_root: &VirtualPath,
    msbuild_properties: &BTreeMap<String, String>,
) -> Option<BTreeMap<String, String>> {
    let file = eval_cache_file(workspace_root, project_id);

    if !file.exists() {
        return None;
    }

    let entry: EvalCacheEntry = serde_json::from_str(&fs::read_file(&file).ok()?).ok()?;

    (entry.digest == eval_cache_digest(project_root, workspace_root, msbuild_properties))
        .then_some(entry.packages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;

    #[test]
    fn framing_distinguishes_content_moved_between_config_files() {
        // The collision being guarded against: these two distributions of the
        // same bytes are indistinguishable once concatenated, and
        // `find_config_files` sorts these two file names adjacently.
        assert_eq!(
            content_digest(&format!("{}{}", "<A/><B/>", "")),
            content_digest(&format!("{}{}", "<A/>", "<B/>")),
        );

        let sandbox = create_empty_sandbox();
        let root = VirtualPath::new(sandbox.path());
        let no_properties = BTreeMap::new();

        sandbox.create_file("Directory.Build.props", "<A/><B/>");
        sandbox.create_file("Directory.Packages.props", "");

        let before = eval_cache_digest(&root, &root, &no_properties);

        // A routine CPM migration: move the declaration to the other file.
        sandbox.create_file("Directory.Build.props", "<A/>");
        sandbox.create_file("Directory.Packages.props", "<B/>");

        assert_ne!(
            before,
            eval_cache_digest(&root, &root, &no_properties),
            "moving a declaration between config files must change the digest"
        );
    }

    #[test]
    fn msbuild_properties_invalidate_the_digest() {
        let sandbox = create_empty_sandbox();
        let root = VirtualPath::new(sandbox.path());

        sandbox.create_file("Directory.Build.props", "<A/>");

        let without = eval_cache_digest(&root, &root, &BTreeMap::new());
        let with = eval_cache_digest(
            &root,
            &root,
            &BTreeMap::from([("SkipApiClientGen".to_owned(), "true".to_owned())]),
        );

        // A conditional PackageReference gated on the property would evaluate
        // differently, so a cached set from one configuration must never be
        // served under the other.
        assert_ne!(without, with);

        let changed_value = eval_cache_digest(
            &root,
            &root,
            &BTreeMap::from([("SkipApiClientGen".to_owned(), "false".to_owned())]),
        );

        assert_ne!(with, changed_value);
    }
}
