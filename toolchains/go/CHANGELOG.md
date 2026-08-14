# Changelog

## 1.5.0

#### 🚀 Updates

- Reworked relationship inference to match package import paths instead of module paths. Each project now resolves a canonical import path (nearest `go.mod` module path plus the project's relative directory), and `go list -deps` results are matched against those by longest prefix. This makes relationships resolvable in repositories that share a single `go.mod` across all projects.
- Sibling modules required by version without a `go.work` no longer create project relationships, since those builds consume the published module rather than the local source. When the `go` binary is unavailable, projects with their own `go.mod` under a workspace `go.work` fall back to resolving relationships from their direct requires.
- `replace` directives keep their meaning in the new model: a require replaced by a local directory always links to the project at that location (it consumes local source even without a `go.work`), while a require replaced by another module never links.
- Imports within a project's own import path are treated as ownership rather than dependencies. `go list -deps ./...` enumerates packages belonging to projects nested inside the scanned project, which previously inferred an edge from the parent to every nested child — forming a cycle whenever a child declared `dependsOn` on its parent.

## 1.4.7

#### 🐞 Fixes

- Fixed project relationships not linking when a `go.mod` is located in a major
  version folder (`v2`+) but its `module` directive omits the version suffix.
  The suffix is now inferred from the folder name, for both the project's alias
  and dependency matching.
- `replace` directives are now honored when linking project relationships. A
  replacement pointing to a local directory links to the project at that
  location (regardless of module names), while a replacement pointing to
  another module no longer creates a relationship.
- Project relationships now reference the dependency's project identifier
  instead of its module path alias, aligning with other toolchains.

## 1.4.6

#### 🐞 Fixes

- Fixed configured `bins` not being reinstalled when their binaries were
  uninstalled or deleted outside of moon. Missing binaries are now detected in
  the globals directory, and only missing binaries are installed. Binaries
  pinned to a version, branch, or commit are always installed, as their
  installed version cannot be verified.
- The `force` option for `bins` entries is now respected, and will always
  install the binary.

## 1.4.5

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 1.4.4

#### 🐞 Fixes

- Fixed `go list -deps` running on non-Go projects.

## 1.4.3

#### 🐞 Fixes

- Fixed `go list -deps` relationship inference not detecting sibling workspace modules imported via subpackages (e.g. `example.com/org/a/pkg`). The command now emits owning module paths using `-f {{if .Module}}{{.Module.Path}}{{end}}` instead of package paths.

## 1.4.2

#### 🐞 Fixes

- Fixed failing release process.
- 
## 1.4.1

#### 🐞 Fixes

- Fixed a `go.mod` parsing regression that failed to parse `tool ()`.

## 1.4.0

#### 🚀 Updates

- Updated `go list --deps` relationship inference to scan all packages (`./...`) by default, so dependencies imported only from subdirectories (`internal/`, `pkg/`, ...) are now inferred.
- Added an `inferRelationshipsPackages` setting to customize the package patterns passed to `go list --deps`.

## 1.3.0

#### 🚀 Updates

- Added support for Go v1.24 `ignore` in `go.mod` and `go.work`.

## 1.2.0

#### 🚀 Updates

- Updated `go list` to not require `go.mod` file to run.

## 1.1.2

#### 🐞 Fixes

- Fixed an issue where `go list` was not running in the project root.
- Fixed an issue where `go list` would add a project dependency to itself.

## 1.1.1

#### 🚀 Updates

- Added `inferRelationships` and `inferRelationshipsFromTests` settings to control `go list --deps` usage.

## 1.1.0

#### 🚀 Updates

- Will now run `go list --deps` to determine project relationships while extending the project graph.

## 1.0.3

#### 🚀 Updates

- Updated with latest moon v2 plugin APIs.

## 1.0.2

#### 🐞 Fixes

- API compatibility.

## 1.0.1

#### 🚀 Updates

- Updated with moon v2 plugin APIs.

#### 🐞 Fixes

- Fixed indirect `go.mod` dependencies being considered a project dependency.

## 1.0.0

#### 🚀 Updates

- Official major release for moon v2.

## 0.2.0

#### 🚀 Updates

- Updated to support moon v1.41.

## 0.1.6

#### 🐞 Fixes

- Fixed `go.mod` parsing failures when `tool` is a list.

## 0.1.5

#### 🚀 Updates

- Removed globals directory injection as this happens in moon directly.

## 0.1.4

#### 🐞 Fixes

- Fixed `bins` failing to install multiple in parallel.

## 0.1.3

#### 🐞 Fixes

- Fixed a "wasm `unreachable` instruction executed" error.

## 0.1.2

#### ⚙️ Internal

- Enabled experimental trace logging.
- Updated dependencies.

## 0.1.1

#### 🐞 Fixes

- Fixed `go.*` parsing failures when there was no trailing newline.

## 0.1.0

#### 🚀 Updates

- Initial release!
