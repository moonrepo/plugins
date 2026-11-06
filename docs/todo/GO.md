# Go toolchain fix plan

Fixes from the Go toolchain audit (2026-11-06, against moon v2.6.5 source, live-checked with moon
2.6.4, proto 0.63.1 and Go 1.26.2). Covers `toolchains/go` (`go_toolchain` 1.5.3) and `tools/go`
(`go_tool` 0.16.8), which the toolchain re-exports as its tier 3 API. Item numbers (#1–#21) match
the audit findings.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build.

```bash
cargo build -p go_toolchain -p go_tool --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p go_toolchain -p go_tool --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

Several findings only show up when moon itself drives the plugin. They were checked in scratch git
repos with `go: { plugin: 'file://<target>/wasm32-wasip1/release/go_toolchain.wasm' }` in
`.moon/toolchains.yml`, run with `~/.proto/tools/moon/2.6.4/moon`:

- **Single module:** a root `go.mod`, with projects `apps/a` (imports `libs/b`) and `libs/b`
  (imports `rsc.io/quote`).
- **Workspace:** a `go.work` using `./a` and `./b`, where `a` requires `rsc.io/quote v1.5.1` and `b`
  requires `v1.5.2`.

Isolate Go state with a scratch `GOPATH` and `GOMODCACHE`, and set `GOTOOLCHAIN=local` unless a test
needs toolchain switching.

Phase 1 was re-checked live with the rebuilt plugin: the workspace installed once with
`go mod download` and left `go.mod` untouched over three runs, `~/go/bin` was on `PATH` without a
`version`, and with `version: '1.26.2'` and an inherited `GOROOT` for 1.26.0, tasks ran 1.26.2 with
`GOROOT` set to it.

**Checked, no change needed:** the `go.mod`, `go.sum` and `go.work` parsers handled all ~1,300 files
in the local module cache. gomod-parser 0.5.3 supports the `tool`, `godebug`, `ignore` and
`toolchain` directives. version_spec parses pseudo-versions and `+incompatible`. The `.sha256`
checksum files exist for every archive.

## Decisions needed

| Decision                                                              | Recommendation                                                   |
| --------------------------------------------------------------------- | ---------------------------------------------------------------- |
| D1 (#18): should `docker prune` delete a committed `vendor/`?         | No. It's source, and deleting it forces downloads in the image   |
| D2 (#13): pin the task toolchain with `GOTOOLCHAIN=local`?            | Yes, when `version` is set and the host hasn't set `GOTOOLCHAIN` |
| D3 (#14): treat every directory under a lone `go.mod` as a member?    | Yes, with `**/*` members                                         |
| D4 (#15): hash the root `go.mod`'s requires for projects without one? | Yes, in `hash_task_contents`                                     |
| D5 (#16): check pinned bins against the installed version?            | Yes, with `go version -m`, for exact semver pins only            |
| D6 (#17): key the project graph cache on `.go` sources?               | Yes, together with #19 so that rebuilds stay cheap               |

## Phase 1: clear-cut fixes

### #1 Stop treating `vendor/` as the install directory

- [x] Drop `vendor_dir_name`
- [x] Skip the download when `vendor/modules.txt` exists
- [x] Tests
- [x] Changelog

**Problem:** `register_toolchain` sets `vendor_dir_name: Some("vendor")` (`tier1.rs:24`). moon reads
that directory as "dependencies are installed", but Go installs modules into the global module
cache, and `vendor/` only exists in repos that commit `go mod vendor` output. In every other repo,
moon treats each run as a first install (moon `install_dependencies.rs:100`):

- The install is forced past the fingerprint (`:174`). In the live check, three runs in a row each
  logged "Installing Go dependencies".
- The affected-mode skip outside CI is bypassed (`:106`).
- The dedupe step is always skipped (`:231`), so `tidyOnChange` has never run since moon v2.4.5.
- `moon docker scaffold` excludes `vendor/**` from both phases (moon `toolchain_plugin.rs:318`). In
  the live check, a committed `vendor/` was missing from both the configs and sources skeletons.

**Change:**

- Return `vendor_dir_name: None`.
- In `install_dependencies`, return no install command when `<root>/vendor/modules.txt` exists. Go
  1.14+ builds from `vendor/` by default, so the download is unnecessary. This replaces moon's
  "vendor exists in CI" skip (`install_dependencies.rs:126`).

`go mod vendor` doesn't copy dependency `go.mod` files (confirmed), so dropping moon's
`!**/vendor/**` manifest exclusion is harmless.

**Tests** (`tier2_test.rs`): no install command when `vendor/modules.txt` exists. Update the tier 1
metadata test.

**Changelog:** `tidyOnChange` now runs `go mod tidy` after `go.mod` or `go.sum` changes outside CI.
Users who enabled it never saw it run.

### #2 Install workspaces with `go mod download`, not `go work sync`

- [x] Switch the command
- [x] Update tests

**Problem:** `install_dependencies` runs `go work sync` for a `go.work` root (`tier2.rs:161-167`).
That command isn't a download. Go documents it as syncing "the workspace's build list back to the
workspace's modules", which upgrades each member's requires. In the live check, `moon run a:hi`
rewrote `a/go.mod` from `rsc.io/quote v1.5.1` to `v1.5.2` and changed `a/go.sum`. That leaves CI
checkouts dirty, and `docker prune`'s production install (moon `prune.rs:181`) edits sources inside
the image.

**Change:** run `go mod download` at the root in both cases. In workspace mode it downloads the
workspace build list (confirmed with Go 1.26.2), and moon keeps a single install per workspace.

**Tests:** `sets_install_command_for_go_work_if_enabled` and `workspaces_take_precedence` expect
`go mod download`.

### #3 One `go install` per bin

- [x] Emit a command per bin
- [x] Run it with `GOBIN` set to the globals directory
- [x] Tests

**Problem:** `setup_environment` groups bins by the first three path segments plus version
(`tier2.rs:298-305`, `get_base_module` at `:340`). Go requires every package in one `go install` to
come from the same module, and module paths don't follow that 3-segment pattern. The common
`golang.org/x/tools/gopls@latest` and `golang.org/x/tools/cmd/goimports@latest` share the key
`golang.org/x/tools@latest`, but are separate modules. Confirmed with Go 1.26.2: installing
`golang.org/x/tools/gopls@v0.21.0` with `golang.org/x/tools/cmd/stringer@v0.21.0` fails with "All
packages must be provided by the same module". The whole group fails. A versionless entry and an
`@latest` one from the same repo also share a key, and Go rejects that mix too ("all arguments must
refer to packages in the same module at the same version").

**Change:**

- Emit one `go install` per bin and remove `get_base_module`. Go's build cache makes the extra
  process cheap.
- Set `GOBIN=<globals_dir>` on each command. proto picks the first lookup directory that already
  has files, so with a fresh `GOBIN` and a populated `~/go/bin`, the plugin checks one directory
  while `go install` writes to another.

**Tests:** replace `separates_commands_by_version` and `separates_commands_by_args` with a
command-per-bin case, and add the gopls plus goimports pair.

### #4 Docker default image for partial versions and requirements

- [x] Use `to_partial_string`
- [x] Tests

**Problem:** `define_docker_metadata` formats `golang:{version}` with `to_string()`
(`tier1.rs:66-73`). moon's tier 3 docs use `version: '1.24'`, which becomes `golang:~1.24`, and
`moon toolchain add go` writes `^1.22.0` (detected from `go.mod`), which becomes `golang:^1.22.0`.
Docker rejects both as invalid references.

**Change:** use `to_partial_string()`, so these become `golang:1.24` and `golang:1.22.0`. Map aliases
other than `latest` (`stable`, `canary`) to `golang:latest`, as Rust #8 did.

**Tests** (`tier1_test.rs`): `1.24`, `~1.24`, `^1.22.0`, `1.26.2`, `latest` and unset.

### #5 Put the Go bin directory on PATH when no version is configured

- [x] Return the globals directory
- [x] Tests

**Problem:** `gather_shared_paths` returns no paths when `globals_dir` resolves (`tier2.rs:60-66`).
moon always passes one (moon `toolchain_plugin.rs:450`), because proto falls back to `$HOME/go/bin`
and creates it. The `GOBIN`/`GOPATH` logic below it therefore only runs in tests. moon adds the
globals directories itself only when `version` is set (`augmented_command.rs`
`inherit_from_toolchains`). In the live check, a task with no `version` had no Go directory on
PATH, so binaries from `bins` weren't found unless the user's shell already had `~/go/bin`.

**Change:** push `globals_dir` into `paths` when it's present, and keep the env fallback. `GOPATH`
can be a list, and `go install` uses the first entry, so split it before adding `bin`.

**Tests:** a provided `globals_dir` is returned for both task commands and scripts.

### #6 An inherited `GOROOT` can replace or break moon's Go

- [x] Remove `$GOROOT/bin` from the globals lookup
- [x] Add `activate_environment` with `GOROOT`
- [x] Tests

**Problem:** `locate_executables` lists `$GOROOT/bin` as a globals directory
(`tools/go/src/proto.rs:219`). `go install` never writes there; it's the Go installation itself.
moon puts globals directories before the tool's own `bin` (moon `toolchain_plugin.rs:232`). Live
check: with `version: '1.26.2'` and `GOROOT` pointing at 1.26.0 (as goenv or old setup guides do),
the task ran `go1.26.0`. When PATH does resolve to the right binary, a mismatched `GOROOT` breaks
builds: `compile: version "go1.26.0" does not match go tool version "go1.26.2"`.

**Change:**

- Drop `$GOROOT/bin` from `globals_lookup_dirs`.
- Add `activate_environment` to `tools/go`, returning `GOROOT=<tool_dir>`, as `tools/java` does with
  `JAVA_HOME`. proto applies it in `proto run` (which shims use) and `proto activate`. moon applies
  it to tasks when `version` is set, without overriding a task's own `GOROOT`
  (`augmented_command.rs:405`).

**Tests:** `tools/go` tests for the lookup dirs and `activate_environment`.

### #7 Go prereleases from 1.21 on can't be downloaded

- [x] Fix `to_go_version`
- [x] Tests

**Problem:** `to_go_version` returns `version.to_string()` for 1.21 and later (`version.rs:33-35`),
so `1.27.0-rc1` becomes `go1.27.0-rc1.<os>-<arch>.tar.gz`, which returns 404. Go publishes
`go1.27rc1…`, which returns 200. proto lists these versions (`1.26.0-rc1` through `1.27.0-rc3`), so
installing any of them fails. The same function builds the GitHub tag for source builds. The test at
`version.rs:105` is commented out.

**Change:** for prereleases, format as `{major}.{minor}{pre}` when the patch is 0, for every Go
version. Keep the 1.21+ branch for stable releases.

**Tests:** enable the commented assertion and add `1.27.0-rc1` → `1.27rc1`.

### #8 Building from source produces an unusable Go

- [x] Use `make.bash` / `make.bat`
- [x] Keep the whole tree

**Problem:** `build_instructions` runs `all.bash` (`proto.rs:115-119`), which builds Go and then runs
its full test suite. That takes many minutes and can fail on host-specific tests. It then removes
everything except `bin` (`:125`). Go needs `src`, `pkg/tool`, `lib` and `go.env` in its GOROOT; a
copy that only keeps `bin/` fails with `cannot find GOROOT directory: 'go' binary is trimmed and
GOROOT is not set` (confirmed).

**Change:** run `make.bash` / `make.bat` and drop `RemoveAllExcept`. Release tags include `VERSION`,
so the `git` requirement can go too; keep `go` for bootstrapping. Not tested end to end, since a
full build is slow.

### #9 One unresolvable import drops all of a project's edges

- [x] Pass `-e` to `go list`
- [x] Warn on failure
- [x] Test

**Problem:** `execute_go_list` returns no imports on a non-zero exit (`package_graph.rs:35-37`).
Confirmed: an import Go can't resolve, such as a new third-party package before `go get`, makes
`go list -deps` exit 1 while stdout still lists every other dependency. The project loses all its
edges, and the graph cache (#17) keeps them lost until `go.mod` changes. With `-e`, the same command
exits 0 and lists the same packages.

**Change:** add `-e`. If the exit is still non-zero, `host_log!` a warning with the first line of
stderr.

**Tests** (`tier2_test.rs` `go_list`): a project with an unresolvable import keeps its local edges.

**Done:** stdout is now parsed even when the exit code is non-zero, after logging the warning.

### #10 Prefer the `toolchain` line in version files

- [x] Parse `toolchain`
- [x] Tests
- [x] Changelog

**Problem:** `parse_version_file` reads only the `go` line (`tools/go/src/proto.rs:61-69`). Go treats
`toolchain go1.X.Y` as the minimum toolchain to run and downloads it itself when the running Go is
older (`GOTOOLCHAIN=auto`). A `go.mod` with `go 1.22` and `toolchain go1.26.0` gives proto
`^1.22.0`, so proto can pick an installed 1.22 that Go then replaces with a download outside
proto.

**Change:** when both lines exist, return `^` plus the `toolchain` version, converted with
`from_go_version`. Skip names that aren't `go<version>`, such as `default`.

**Tests:** `tools/go/tests/versions_test.rs`: `go` only, `go` plus `toolchain`, and an unknown
`toolchain` name.

**Changelog:** proto and `moon toolchain add` can now resolve a newer Go for the same `go.mod`.

**Done:** the `toolchain` line wins whenever it's a version. It isn't compared with the `go` line,
since Go only writes a `toolchain` line when it's newer.

### #11 Docker scaffold: copy the `go.mod` files `go mod download` needs

- [x] Copy `go.work` members' `go.mod` and `go.sum`
- [x] Copy local `replace` targets' `go.mod` and `go.sum`
- [x] Tests

**Problem:** the configs phase copies manifests only from project roots and the workspace root.
`go mod download` (#2) loads every `go.work` member and every local `replace` target. Confirmed: when
one is missing, the download fails with `cannot load module b listed in go.work file` or
`reading ../third_party/lib/go.mod: no such file`. This happens in Docker whenever such a directory
isn't a moon project, such as a `tools` module.

**Change:** add `scaffold_docker`. In the configs phase, copy `go.mod` and `go.sum` for every `use`
directory (in the root call, where `project` is `None`) and for every local `replace` path inside
the workspace (in project calls). The sources phase is unchanged, so a replaced directory that isn't
a project still has no sources; document that it should be a project.

**Tests** (`tier1_test.rs`): a `go.work` with a non-project member, and a `replace` to a non-project
directory.

**Done:** a `go.work` is read in project calls too, not just the root call, since a workspace can
live in a project. Live check: `moon docker scaffold` copied a non-project `tools/go.mod`, and
`go mod download` then succeeded in the configs skeleton.

### #12 Small fixes

- [x] `locate_dependencies_root` (`tier2.rs:141-144`): drop the `go.sum` lookup. A nested module
      with a `go.mod` but no `go.sum` resolves to the parent module's `go.sum`. `go.sum` always sits
      beside its `go.mod`, so the nearest `go.mod` is always right.
- [x] `GoWork::parse` (`go_work.rs:42-82`): trim lines instead of matching a tab or four spaces,
      strip quotes, and accept `use(`. Confirmed against `go list -m`: two-space indentation and
      `use(` dropped modules, and `"./quoted"` was reported with its quotes. **Since replaced:**
      `go.work`, `go.sum` and `go.work.sum` now use winnow grammars (`syntax.rs`), where `//` only
      starts a comment at the beginning of a token. The old `go.sum` parser cut off 28 checksums in
      the local cache that contain `//`, like `golang.org/x/net v0.10.0`; on 305 real files, that
      was the only difference.
- [x] `GoSum::parse` keeps the last version per module (`go_sum.rs:45`). 20 of the 287 `go.sum`
      files in the local cache list stale zip hashes for older versions. Keep every entry. This is
      cosmetic for hashing, since Go module versions are immutable.
- [x] `parse_manifest` skips `// indirect` requires (`tier2.rs:224-226`). Since Go 1.17 they're part
      of the build, so a bump to one never changes a task hash when `go.mod` isn't an input. Report
      them too. This changes Go task hashes once.
- [x] Honor `GOWORK` from the host env in `locate_dependencies_root` and `install_dependencies`: Go
      ignores `go.work` when `GOWORK=off` and uses the named file otherwise. **Done:** only `off`
      is handled; a `GOWORK` path to another file is ignored.
- [x] `download_prebuilt` and `build_instructions`: add FreeBSD arm64, and Linux `Riscv64`,
      `Powerpc64` (`ppc64le`) and `LongArm64` (`loong64`). All four 1.26.2 archives exist.
- [x] Remove the unused `exec_command` host function (`tools/go/src/proto.rs:9-12`) and the stray
      backtick in the `version` doc comment (`config.rs:35`).

## Phase 2: after decisions

### #13 Make `go.version` control which Go runs tasks (D2)

- [ ] Set `GOTOOLCHAIN=local` for tasks
- [ ] Set it on setup and install commands
- [ ] Changelog

**Problem:** since Go 1.21, `go` switches toolchains on its own: when `go.mod` or `go.work` requires
a newer Go, it downloads that release into the module cache and runs it. Confirmed: with `go 1.27.0`
in `go.mod` and Go 1.26.2 running, `go version` prints `go: downloading go1.27.0`. moon then runs
tasks with a Go it didn't install and hashes the configured version instead. This is the Go version
of Rust #3.

**Recommended:** when `version` is set and `get_host_env_var("GOTOOLCHAIN")` is empty, set
`GOTOOLCHAIN=local` in `extend_task_command` and `extend_task_script`, and on the `go install` and
`go mod download` commands. A `go.mod` that needs a newer Go then fails with `go.mod requires go >=
1.27.0 (running go 1.26.2; GOTOOLCHAIN=local)`, so the user can bump `version`. Don't set it for
`go list` in `extend_project_graph`, which runs before setup; an error there would drop edges.

**Alternative:** set it in `tools/go`'s `activate_environment` (#6), which would apply it to every
proto user. That's broader than this audit.

**Changelog:** needs a callout, since mismatched repos now fail instead of downloading.

### #14 Single-module repos get no install or setup action (D3)

- [ ] Report members for a lone module root
- [ ] Make `parse_manifest` tolerate invalid files
- [ ] Tests

**Problem:** without a `go.work`, `locate_dependencies_root` reports no `members`. moon's
`in_dependencies_workspace` then only accepts the root itself (moon `toolchain_plugin.rs:164`), so
projects below it get no `InstallDependencies` and no `SetupEnvironment` action
(`action_graph_builder.rs:424`). Live check: with `bins` configured, `moon run a:hello` only added
`SetupToolchain(go)`, so bins are installed only by `moon setup`. CHANGELOG 1.5.0 added support for
single-module repos, a common Go layout.

**Recommended:** when the root comes from a `go.mod` (not a `go.work`), report `members:
["**/*"]`. Nested modules resolve to their own `go.mod` first, so they aren't affected. Commands for
a workspace root run without a project, which gets every configured toolchain's env since moon
v2.5.0 (`augmented_command.rs:126`), so no `PROTO_GO_VERSION` workaround is needed.

**Cost:** when a `go.mod` changes, moon globs members for manifests (`install_dependencies.rs:333`),
which walks the whole module. `testdata` directories can hold intentionally invalid `go.mod` files,
so `parse_manifest` should log a warning and return nothing instead of failing the install.

**Alternative:** a moon change that treats every directory under a member-less root as covered, for
toolchains that opt in.

### #15 Single-module task hashes miss dependency bumps (D4)

- [ ] Hash the owning module's requires
- [ ] Tests

**Problem:** moon only hashes dependencies from a manifest in the project's own directory (moon
`task_hashing.rs:373`). Projects in a single-module repo have none, so `go get -u` changes nothing
in their task hashes unless the task lists `/go.mod` or `/go.sum` as inputs, and moon's Go docs
don't suggest that.

**Recommended:** in `hash_task_contents`, when the project has no `go.mod`, find the nearest one up
to the workspace root (reusing `module_dir_candidates`) and add its `go`, `toolchain`, and every
`require` (including indirect) as `module@version`. Projects that own a `go.mod` are already covered
by moon (after #12's indirect change). Not covered: in a `go.work`, a member's build list can change
through another member's requires.

**Tests:** a project below the root module gets the root's requires; a project with its own
`go.mod` gets none.

### #16 Check pinned bins with `go version -m` (D5)

- [ ] Compare installed module versions
- [ ] Update tests

**Problem:** `is_bin_installed` (`tier2.rs:243-264`) always includes bins pinned to a version. moon
fingerprints the returned commands (`setup_environment.rs:101`), so an unchanged command list never
re-runs, and a pinned binary that was deleted or replaced isn't reinstalled. CHANGELOG 1.4.6 says
such bins are "always installed", which isn't true.

**Recommended:** for exact semver pins (`@v1.2.3`), run `go version -m <globals_dir>/<bin>` and
compare the `mod` line's version; output was confirmed on `~/go/bin/gopls`. Include the bin when the
file is missing, isn't a Go binary, or the versions differ. Branch and commit pins keep today's
behavior. This is one process per pinned bin per setup.

**Tests:** flip `always_installs_versioned_bins` to skip a matching install. Use a stub binary
script, since `go version -m` needs a real Go binary, or skip without `go`.

### #17 Go source edits don't refresh the project graph (D6)

- [ ] Report Go sources as graph inputs when `go list` is used
- [ ] Changelog

**Problem:** edges come from `import` statements, but moon caches the graph by moon configs,
manifests and the plugin's `input_files` (moon `workspace_cache.rs:200`), which are only `go.mod` and
`go.sum`. In a single-module repo, a new import between projects never touches `go.mod`. Live check:
after `apps/a` started importing `libs/b`, `moon project a` still showed no dependencies after
committing, and only showed `b` once `go.mod` was edited. Affected detection and `^:` deps are wrong
until then, including in CI runs that restore `.moon/cache`.

**Recommended:** when `go` exists and inference is on, add the `.go` files under each project to
`input_files`, skipping nested modules, `vendor`, `testdata`, and directories starting with `.` or
`_`, as Go does. Each Go edit then rebuilds the graph, which runs `go list` once per project today,
so land #19 first or together.

**Not covered:** moon hashes the input list saved by the previous build, so a new `.go` file isn't
seen until another input changes. That needs glob inputs from moon (see "Outside this repo").

### #18 Leave committed `vendor/` alone in `docker prune` (D1)

- [ ] Stop deleting `vendor/`
- [ ] Deprecate `vendorDir`

**Problem:** after #1, scaffolding keeps `vendor/`, but `prune_docker` still deletes it when
`deleteVendorDirectories` is on (`tier1.rs:79-93`), which is moon's default. The production install
then downloads every module into the image. Unlike `node_modules`, `vendor/` is committed source that
repos keep for offline, reproducible builds, and the module cache isn't any smaller.

**Recommended:** make `prune_docker` a no-op and deprecate `vendorDir`. Go only builds from
`vendor/` at the module root anyway, so the setting adds nothing.

## Phase 3: lower priority

### #19 One `go list` per module

- [ ] Batch inference per module root

`dependencies_from_go_list` runs `go list -deps` once per project, and twice with test inference
(`package_graph.rs:420-470`). Each call loads the whole module, and `-deps` gives transitive edges:
if A imports B and B imports C, A also gets an edge to C. A single
`go list -e -f '{{.ImportPath}} {{.Imports}} {{.TestImports}} {{.XTestImports}}' <patterns>` per
module root gives every package's direct imports, production and test, in one call. Map each
package to its project by the existing longest-prefix match. That's needed for #17, and makes edges
direct, as in the other toolchains. `inferRelationshipsPackages` still sets the patterns.

### #20 Hash the target platform, not just the host

- [ ] Read Go's target env vars

`hash_task_contents` hashes the host OS, arch and libc (`tier2.rs:324-338`). Go's output depends on
`GOOS`, `GOARCH`, `CGO_ENABLED`, `GOFLAGS`, `GOEXPERIMENT` and the per-arch variables (`GOAMD64`,
`GOARM64` and so on). moon hashes task-level env, but a host-level `GOOS=linux` for a cross build
doesn't change the hash. Add the ones that are set on the host.

### #21 Detection names

- [ ] `exe_names` (`tier1.rs:21`): add `golangci-lint`, `gotestsum`, `staticcheck` and `govulncheck`.

## Not planned

- **Rewriting bare tool names to `go tool <name>`:** Go 1.24's `tool` directive pins tools in
  `go.mod`, and `go tool <name>` is short and explicit. Document it as the alternative to `bins`
  instead of rewriting commands, as Rust and Ruby do.
- **Syncing `go.mod`'s `toolchain` line from `version`:** #13 already enforces the configured
  version, and editing `go.mod` is more invasive than `rust-toolchain.toml`. Revisit if users ask
  for it.
- **Loading versions from `go.dev/dl/?mode=json`:** the git tags work, and checksums already come
  from the `.sha256` files.

## Outside this repo

- [ ] moon: graph cache inputs are exact paths, so a new source file can't invalidate the graph
      (#17). Let plugins return input globs, such as `apps/a/**/*.go`.
- [ ] moon `website/docs/__partials__/create-task/go/inputs.mdx`: `src/**/*` isn't a Go convention.
      Use `**/*.go`, and mention `/go.mod` and `/go.sum` for single-module repos until #15 lands.
- [ ] moon `website/docs/config/toolchain.mdx`: the Go section only says to run `moon toolchain info
      go`. Document `GOTOOLCHAIN` (#13), `go tool`, and that `bins` installs into `GOBIN`.
- [ ] moon `detect_task_usage`: the proto-binary branch never matches (see `RUST.md`).

## Release

- `go_toolchain`: Phase 1 changes when installs run, stops `go.mod` rewrites, and makes
  `tidyOnChange` work, so ship it as 1.6.0 with changelog callouts. Phase 2 changes which Go runs,
  task hashes and graph inputs, so ship it as 1.7.0 with callouts.
- `go_tool`: #6, #7, #8 and #10 change installs and version detection, so ship them as 0.17.0. The
  toolchain re-exports `go_tool`, so tool fixes also need a toolchain release.
