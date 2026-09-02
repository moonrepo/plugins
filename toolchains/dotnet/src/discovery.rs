//! Locating the files this toolchain cares about, relative to a directory.
//!
//! The `find_*` helpers enumerate one directory; `walk_up` supplies the
//! workspace-bounded upward traversal they are usually driven by. Neither has a
//! PDK equivalent: `warpgate_pdk` exposes no directory listing at all, and
//! `moon_pdk::locate_root*` walks up without a bound.

use moon_pdk_api::VirtualPath;
use starbase_utils::fs;

/// Project file extensions this toolchain understands.
pub const PROJECT_EXTENSIONS: &[&str] = &["csproj", "fsproj", "vbproj"];

/// Directories never worth descending into: build output, or owned by another
/// tool. Shared with tier 3's `global.json` scan.
pub const SKIP_DIRS: &[&str] = &["bin", "obj", "node_modules", ".git", ".moon"];

/// Workspace-level MSBuild/NuGet config files that can change evaluation,
/// restore, or build behavior from any level between a project dir and the
/// workspace root. Matched case-insensitively: NuGet itself accepts any
/// casing of `nuget.config`, and over-matching the others merely over-hashes
/// (a spurious cache invalidation, never a stale hit).
pub const CONFIG_FILE_NAMES: &[&str] = &[
    "directory.build.props",
    "directory.build.rsp",
    "directory.build.targets",
    "directory.packages.props",
    "global.json",
    "nuget.config",
];

/// Does a file name carry one of the given extensions (case-insensitively)?
fn has_extension(name: &str, extensions: &[&str]) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| {
        extensions
            .iter()
            .any(|known| known.eq_ignore_ascii_case(ext))
    })
}

/// Files directly inside a directory whose name satisfies `keep`, sorted by
/// name and returned as paths under `dir`.
///
/// An unreadable directory yields nothing rather than an error. Every caller
/// treats "nothing matched" and "could not look" identically, and propagating
/// would turn a single unreadable subdirectory into a failed
/// `install_dependencies` (via `contains_lockfile`'s recursion) or force the
/// infallible digest and boolean-probe callers to change shape. Note that
/// `fs::read_dir` already maps a missing directory to an empty list, so this
/// only absorbs real I/O failures — permissions, symlink loops, WASI
/// `NotCapable`.
fn list_files(dir: &VirtualPath, keep: impl Fn(&str) -> bool) -> Vec<VirtualPath> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };

    let mut names = entries
        .into_iter()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| keep(name))
        .collect::<Vec<_>>();

    names.sort();

    names.into_iter().map(|name| dir.join(name)).collect()
}

/// Names of the subdirectories directly inside a directory. Same
/// unreadable-yields-nothing contract as [`list_files`].
fn list_dirs(dir: &VirtualPath) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };

    entries
        .into_iter()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// List MSBuild project files (*.csproj etc.) directly inside a directory
/// (non-recursive).
pub fn find_project_files(dir: &VirtualPath) -> Vec<VirtualPath> {
    list_files(dir, |name| has_extension(name, PROJECT_EXTENSIONS))
}

/// NuGet lock file names: the default `packages.lock.json`, plus the
/// `packages.<project>.lock.json` convention used when `NuGetLockFilePath`
/// renames it (case-insensitive, NuGet accepts any casing).
pub fn is_lock_file_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    // The default name needs no special case: it starts with `packages.` and
    // ends with `.lock.json` like the renamed variants.
    lower.starts_with("packages.") && lower.ends_with(".lock.json")
}

/// List NuGet lock files directly inside a directory (non-recursive), sorted.
pub fn find_lock_files(dir: &VirtualPath) -> Vec<VirtualPath> {
    list_files(dir, is_lock_file_name)
}

/// List hash-relevant config files directly inside a directory
/// (non-recursive), sorted by actual file name.
pub fn find_config_files(dir: &VirtualPath) -> Vec<VirtualPath> {
    list_files(dir, |name| {
        CONFIG_FILE_NAMES.contains(&name.to_ascii_lowercase().as_str())
    })
}

/// Does a directory directly contain a solution file (*.sln / *.slnx)?
pub fn has_solution_file(dir: &VirtualPath) -> bool {
    !list_files(dir, |name| has_extension(name, &["sln", "slnx"])).is_empty()
}

/// How far below a dependencies root to look for a lock file. Lock files sit
/// next to each project file rather than at the root, and .NET repositories
/// conventionally nest them a few levels down (`src/<area>/<project>/`).
pub const LOCKFILE_SEARCH_DEPTH: u8 = 5;

/// Depth-limited search for any NuGet lock file under a directory.
/// Lock files live next to each project file, not at the dependencies root,
/// so a root-only check would miss them.
pub fn contains_lockfile(dir: &VirtualPath, depth: u8) -> bool {
    if !find_lock_files(dir).is_empty() {
        return true;
    }

    if depth == 0 {
        return false;
    }

    list_dirs(dir)
        .into_iter()
        .filter(|name| !SKIP_DIRS.iter().any(|skip| skip.eq_ignore_ascii_case(name)))
        .any(|name| contains_lockfile(&dir.join(name), depth - 1))
}

/// SDK versions laid out under a .NET root (`<root>/sdk/<version>`).
pub fn installed_sdk_versions(root: &VirtualPath) -> Vec<String> {
    list_dirs(&root.join("sdk"))
}

/// SDK roots that proto has installed, newest version first.
///
/// proto's inventory layout is `<store>/tools/<id>/<version>`, mandated by
/// proto core rather than chosen by a plugin, and moon maps proto's store into
/// this plugin as `/proto` (its plugin registry adds proto's virtual paths to
/// every plugin). Each product directory is a *complete* SDK root — muxer,
/// `host/fxr`, `shared/`, `sdk/<band>` — so it serves as `DOTNET_ROOT` as-is.
///
/// This exists because graph building cannot see the task PATH. moon puts the
/// resolved tool directory on a task's PATH, but `extend_project_graph` shells
/// out to `dotnet` from the *host* PATH and is given no tool directory, so
/// without this an SDK proto installed is invisible to dependency and task
/// inference.
pub fn proto_sdk_roots() -> Vec<VirtualPath> {
    let inventory = VirtualPath::new("/proto/tools/dotnet");
    let mut versions = list_dirs(&inventory);

    // Highest version first. Sorting the strings would put 8.0.9 above 8.0.10.
    versions.sort_by_key(|version| std::cmp::Reverse(sort_key(version)));

    versions
        .into_iter()
        .map(|version| inventory.join(version))
        .filter(|root| !installed_sdk_versions(root).is_empty())
        .collect()
}

/// Numeric-aware sort key. Trailing pre-release text is dropped.
fn version_key(version: &str) -> Vec<u32> {
    version
        .split(['.', '-'])
        .map_while(|part| part.parse::<u32>().ok())
        .collect()
}

/// Ordering key: the numeric components, then a release ahead of its own
/// pre-release. Without the second half, `11.0.100` and `11.0.100-rc.2` compare
/// equal and which SDK gets used comes down to directory listing order.
fn sort_key(version: &str) -> (Vec<u32>, bool) {
    (version_key(version), !version.contains('-'))
}

/// Directories from `start` up to and including `workspace_root`.
///
/// Every upward search in this plugin is bounded by the workspace root, which is
/// why moon's own `locate_root*` helpers are not used: they are unbounded, so
/// `global.json` or `dotnet-tools.json` discovery could escape into `$HOME` or a
/// parent repository and pick up a file that governs nothing here. `VirtualPath`
/// gives no such bound of its own: `parent()` keeps yielding directories all the
/// way to the filesystem root.
///
/// Stops early if `start` is not under `workspace_root`, once `parent()` runs
/// out.
pub fn walk_up(
    start: &VirtualPath,
    workspace_root: &VirtualPath,
) -> impl Iterator<Item = VirtualPath> {
    let root = workspace_root.to_owned();
    let mut next = Some(start.to_owned());

    std::iter::from_fn(move || {
        let dir = next.take()?;

        next = if dir == root { None } else { dir.parent() };

        Some(dir)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_versions_numerically_not_lexically() {
        let mut versions = vec![
            "8.0.9".to_owned(),
            "10.0.100".to_owned(),
            "8.0.10".to_owned(),
            "9.0.100".to_owned(),
        ];

        versions.sort_by_key(|version| std::cmp::Reverse(sort_key(version)));

        // Lexical order would put 8.0.9 above 8.0.10, and 10.0.100 below both.
        assert_eq!(versions, ["10.0.100", "9.0.100", "8.0.10", "8.0.9"]);
    }

    #[test]
    fn prefers_a_release_over_its_own_prerelease() {
        let mut versions = ["11.0.100-rc.2", "11.0.100", "11.0.100-preview.7"]
            .map(str::to_owned)
            .to_vec();

        versions.sort_by_key(|version| std::cmp::Reverse(sort_key(version)));

        // On the numeric key alone all three tie, leaving the choice of SDK to
        // whatever order the directory happened to be listed in.
        assert_eq!(versions[0], "11.0.100");
    }

    #[test]
    fn version_key_stops_at_non_numeric_parts() {
        assert_eq!(version_key("11.0.100-preview.7.26381.103"), [11, 0, 100]);
        assert_eq!(version_key("8.0.424"), [8, 0, 424]);
        assert!(version_key("not-a-version").is_empty());
    }
}
