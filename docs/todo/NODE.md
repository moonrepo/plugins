# Node.js toolchain fix plan

Fixes from the Node.js toolchain audit (2026-11-17, against moon v2.6.5 source, live-checked with
moon 2.6.4, proto 0.64.0, Node.js 22.22.3, 24.22.3 and 26.0.0, and npm 11.13.0). Covers
`toolchains/node` (`node_toolchain` 1.0.5, toolchain ID `node`) and `tools/node` (`node_tool`
0.18.0), which the toolchain re-exports as its tier 3 API, plus the Node.js routing in
`toolchains/javascript` (`javascript_toolchain` 1.3.2, `javascript`). The tool was audited on its own
on 2026-11-06, so it's only covered here where the toolchain depends on it. Item numbers (#1–#10)
match the audit findings. At the time of the audit, the latest releases were Node.js 26.13.1
(Lithium, the latest LTS), npm 12.2.2, moon 2.6.5 and proto 0.64.0.

`syncVersionManagerConfig` didn't move into the JavaScript toolchain. plugins#96 was closed by
deprecating it ("Ended up deprecating this"): #181 removed node's `setup_environment`, and nothing
else reads the setting, including `toolchains/javascript` (no `NodeVersionManager` there). #3 covers
what's left of it.

The most visible finding needs moon: tasks that run `node`, or scripts that start with it, put Node's
own `bin` ahead of the npm toolchain's, so `npm` and `npx` in them are Node's bundled copy, not the
configured version. It's in `MOON_COMPACT.md`.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build. Build the tool and the toolchain in
separate runs.

```bash
cargo build -p node_tool --target wasm32-wasip1 --release
cargo build -p node_toolchain --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p node_toolchain -p node_tool -p lang_javascript_common --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

Test coverage is thin. `tests/moon_test.rs` is commented out ("These tests hang for some reason!"),
so nothing runs moon end to end. `tier1_test.rs` only covers `define_docker_metadata` (`latest` and
`20`), and there are no `register_toolchain` or `initialize_toolchain` tests. Nothing in the node
tests mocks host functions.

Findings marked **Live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in a scratch git repo:
an npm workspace with one project, `apps/web` (a `package.json` and a `moon.yml`), and:

```yaml title=".moon/toolchains.yml"
javascript:
  packageManager: 'npm'
node:
  version: '22.22.3'
npm:
  version: '11.13.0'
```

Run moon with a clean environment (`env -i HOME=$HOME PATH=/usr/bin:/bin:/usr/sbin:/sbin
SHELL=/bin/bash`). With a login shell's `$SHELL`, moon runs installs through `zsh -c`, which sources
`~/.zshenv`; on the audit machine that put a vite-plus `npm` first, and installs ran it instead of
proto's. A `preinstall` script that prints `npm_config_user_agent` shows which npm and Node ran the
install. `moon docker setup` was emulated by copying `.moon/docker/configs` to a fresh git repo.

moon 2.6.4 loads the released `node_toolchain` 1.0.5 (moon `crates/config-loader/src/
toolchains_config_ext.rs:20`). Its tier 1 and tier 2 source is identical to `master`, so those
results apply to `master`, but it embeds `node_tool` 0.17.13. Toolchains built from `master` can't
load in moon 2.6.x (see "Release"), so the tool on `master` was checked with proto 0.64.0, which
loads the released `node_tool` 0.18.0, built from the same source. proto treats a working directory
outside `$HOME` differently (see "Outside this repo"), so put scratch repos under `$HOME` unless
testing that.

**Checked, no change needed:**

- **Version files:** the tool reads `.nvmrc`, `.node-version` and `package.json` in each directory,
  nearest first, the same files and order as fnm (`src/version_files.rs`). Live, with proto 0.64.0,
  these resolve: `lts/*` and `lts-*` (latest LTS, 26.13.1), `lts/iron` and `lts-iron` (20.20.2),
  `lts`, `node`, `stable`, `latest`, `krypton`, `v20`, `v20.11.0`, `20.x`, `20 # LTS`, and trailing
  whitespace. In `package.json`, `devEngines.runtime` (object or array) wins over `volta` and then
  `engines`, and ranges like `^22 || ^24`, `>=22 <23`, `20.x || 22.x` and `22.0.0 - 22.22.3`
  parse.
  `lts/-1`, `system`, `iojs`, `current` and `lts/Iron` fail with "Failed to resolve … to a valid
  supported version", which is loud enough (see "Not planned"). nodenv only reads the first word of
  `.node-version` and has no comment support, so proto is more lenient than it. mise reads
  `.nvmrc`, `.node-version` and `devEngines.runtime` (opt-in), but not `engines.node` (#6).
- **moon's init and proto agree:** `moon toolchain add` detects the version with proto's
  `Detector`, which calls the tool's `parse_version_file` (moon `crates/app/src/commands/toolchain/
  add.rs:88`, `toolchain_plugin.rs:394-414`). Live: `.nvmrc` `22.22.3`, `lts/iron` and `v24` wrote
  `22.22.3`, `lts/iron` and `~24`.
- **`executeArgs`** applies only to `node` and `nodejs` task commands, like v1's `binExecArgs`
  (v1.41.8 `legacy/node/platform/src/actions/run_target.rs:109-117`). Live, it's prepended before
  the task's args and isn't applied to `script` tasks, as documented.
- **Globals:** in a task, `npm prefix -g` printed `~/.proto/tools/node/22.22.3`, whose `bin` is on
  PATH when `version` is set. With npm's `shared-globals-dir`, the npm tool sets `PREFIX` to
  `~/.proto/tools/node/globals`, and the node tool's `globals_lookup_dirs`
  (`tools/node/src/proto.rs:474`) covers its `bin`.
- **Installs** run proto's npm with proto's Node. Live: `npm_config_user_agent` was
  `npm/11.13.0 node/v22.22.3`, from `~/.proto/tools/npm/11.13.0`, with 8 combinations of other
  toolchains configured.
- **Corepack:** nothing in this repo runs it. Node 26.0.0's `bin` has only `node`, `npm` and `npx`,
  and the npm, pnpm and Yarn toolchains install package managers through proto, so Node 25+ not
  bundling Corepack breaks nothing here. Node 22.22.3 bundles Corepack 0.34.6, whose auto-pin is
  off. Corepack's shims only matter through the PATH order in `MOON_COMPACT.md`.
- **`vendor_dir_name: node_modules`:** the toolchain isn't tier 2, so moon only uses it to exclude
  `node_modules` from `docker scaffold` (moon `toolchain_plugin.rs:317-319`), which is right.
- **`initialize_toolchain`'s URLs** (`guides/javascript/node-handbook`, `config/toolchain#node`)
  exist in moon 2.6.5's website.
- **ADRs:** `download_prebuilt`'s `unreachable!()` follows `check_supported_os_and_arch`
  (ADR-0005.4), `extract_volta_version`'s `.expect` is on a path's parent (ADR-0005.3), and
  `post_install` checks `proto install npm bundled`'s exit code (ADR-0007.3). No unit tests are
  behind the `wasm` feature (ADR-0006).

## Decisions needed

| Decision                                                                | Outcome                                                                                |
| ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| D1 (#5): copy the files that install lifecycle scripts run into Docker? | **Decided:** yes, from the JavaScript toolchain's `scaffold_docker`, for every manager |
| D2 (#6): keep reading `engines.node`, which in a member beats the root? | **Decided:** yes, as fnm and `n` do; moon's handbook documents the pin                 |
| D3 (#8): default image when `version` is unset?                         | **Decided:** `node:lts` instead of `node:latest`                                       |

The bundled npm on PATH and hashing the resolved version need moon, so they're under "Outside this
repo" rather than here.

## Phase 1: clear-cut fixes

### #1 Read only the version fields from `package.json`

**Done:** `nodejs_package_json` now skips any field it can't parse and keeps an unparsable
dependency as `Unknown`, so an invalid `version` or specifier no longer drops the pin, for every
tool, without a separate struct (`JAVASCRIPT.md` #1, 2026-11-17). The `debug!` and tests followed
on 2026-11-18.

- [x] Parse the version fields without typing the rest of the file (`nodejs_package_json` 0.5)
- [x] Log a `package.json` that isn't JSON
- [x] Tests: `parses_engines_when_other_fields_are_invalid`, `skips_package_json_that_isnt_json`,
      and `extracts_node_versions_when_other_fields_are_invalid` in `lang-javascript-common`

**Problem:** `parse_version_file` deserializes the whole file as `nodejs_package_json::PackageJson`,
and skips the file when that fails (`tools/node/src/proto.rs:64`). `extract_volta_version` does the
same for `volta.extends` (`crates/lang-javascript-common/src/package_json.rs:107`). That struct types
fields the plugin never reads: `version` is a strict `semver::Version`, and every dependency map is
parsed as `VersionProtocol`, which fails on requirements the `semver` crate rejects. **Live** (proto
0.64.0, `node_tool` 0.18.0): with `"engines": { "node": "24.22.3" }`, adding either of these made
proto detect nothing and fall back to the global pin (22.14.0):

- `"version": "1.0"`
- a dependency `"foo": "^v1.2.3"` (or `">=1.0.0-"`)

npm 11.13.0 installs both files without complaint, so the pin disappears silently, for proto and
for `moon toolchain add node`. This breaks ADR-0008.2 (only define the fields the plugin uses) and
ADR-0008.5 (skip the entry, not the file). A `package.json` that isn't JSON is skipped without a
`debug!` (ADR-0008.8).

**Change:**

- Add a pure struct to `lang-javascript-common` holding only the version fields, as `Option`s of
  `JsonValue` with `#[serde(default)]`: `engines`, `devEngines`, `volta` and `packageManager`.
  Port the `extract_*` helpers to it, so that only the `node` entry (or the named package manager)
  goes through `VersionProtocol::parse`, as today. One bad entry then skips that entry
  (ADR-0008.5). `packageManager` is included so the Bun, Deno and node-depman tools can switch to it
  (see "Handed to other plans").
- Use it in `parse_version_file` and for `volta.extends`.
- `debug!` naming the file when it isn't JSON (ADR-0008.8).

**Not covered:** `"engines": { "node": "v22.22.3" }` isn't detected today, as `VersionProtocol`
parses a leading letter as a tag, and `extract_valid_version_protocol` drops tags. npm accepts the
`v`. Parsing the raw string with `UnresolvedVersionSpec` would accept it, but changes how every range
is read, so it's left out.

**Tests:** unit tests in `lang-javascript-common` (ADR-0006) for an invalid `version`, an invalid
dependency specifier and an invalid entry under another engine, each still yielding `engines.node`,
`devEngines.runtime` and `volta.node`. Add the first two to `tools/node/tests/versions_test.rs`.

### #2 Docker image tags for aliases, canary and prereleases

- [x] Map aliases to Docker Hub tags
- [x] Tests

**Done** by the shared `get_docker_image_tag` (`docs/adr/draft-default-docker-image-tags.md`), with
`get_alias_tag` in `tier1.rs` for the aliases. `canary` and pre-releases are `latest`. Codenames
aren't lowercased, as proto doesn't resolve mixed-case aliases like `lts/Iron` either.

**Problem:** `define_docker_metadata` formats `node:{version}` with `to_partial_string()`
(`toolchains/node/src/tier1.rs:61-68`), which returns aliases as they are and canary as `canary`.
**Live** (`moon docker file web --defaults`):

| `node.version`       | `FROM`                    | On Docker Hub   |
| -------------------- | ------------------------- | --------------- |
| `lts/iron`           | `node:lts/iron`           | not a valid tag |
| `stable`             | `node:stable`             | 404             |
| `canary`             | `node:canary`             | 404             |
| `node`               | `node:node`               | 404             |
| `27.0.0-alpha.0.1.1` | `node:27.0.0-alpha.0.1.1` | 404             |
| `<22`                | `node:22`                 | doesn't match   |

Docker Hub's tags API returns 200 for `lts`, `iron`, `lts-iron`, `krypton`, `lithium`, `current`
and `latest`. `moon toolchain add node` writes `version: lts/iron` from an `.nvmrc` containing
`lts/iron` (live), so moon's own init leads to an image that can't be pulled. nodejs.org's release
index has listed 27.0.0 alphas since 2026-10-29, and `proto versions node` lists 3 of them.

**Change:** before formatting:

- `lts/<name>` and `lts-<name>` become `<name>`, lowercased. Docker Hub tags LTS lines by codename,
  like `iron`, `krypton` and `lithium`.
- `lts/*`, `lts-*`, `lts` and `stable` become `lts`. The tool's `stable` alias is the latest LTS
  (`tools/node/src/proto.rs:196-198`).
- `node`, `latest` and `canary` become `latest`.
- A version or requirement with a prerelease becomes `latest`.

Other aliases, like a codename (`iron`) or a proto alias, stay as they are. `<` and `<=`
requirements stay wrong; they're rare for an image.

**Tests** (`tier1_test.rs`): each row above, plus `20`, `~22.22`, `22.22.3` and unset.

### #3 Stop prompting for `syncVersionManagerConfig`

- [x] Remove the prompt
- [x] Describe the setting as unused
- [x] Test: `doesnt_prompt_for_unused_settings`

**Problem:** the setting was deprecated in 1.0.4 and does nothing since #181 removed node's
`setup_environment`. But `initialize_toolchain` still prompts for it, with `nodenv` as the default
and no way to decline (`toolchains/node/src/tier1.rs:32-42`). **Live:** `moon toolchain add node
--yes` wrote `syncVersionManagerConfig: nodenv` every time. With `syncVersionManagerConfig: nvm` and
`version` set, `moon run` wrote no `.nvmrc` and logged nothing. `moon toolchain info node` still
describes it as "syncs the version to the chosen config", with a "(deprecated)" suffix
(`config.rs:38-40`).

**Change:**

- Return no prompts from `initialize_toolchain`.
- Describe the field as "Unused. moon doesn't write version manager files." and keep
  `#[deprecated]`, so existing configs still validate, as Go's `vendorDir` and Python #19 did.
  Remove it, and `NodeVersionManager`, in the next major.

**Tests:** an `initialize_toolchain` test in `tier1_test.rs` asserting no prompts.

### #4 Write profiles to moon's cache, per task

**Not planned (2026-11-18):** the maintainer decided that profiles stay in the project's `.moon`
directory. A project's `.moon/cache` wouldn't be ignored either: the `.moon/cache` entry that
`moon init` writes is anchored to the workspace root (confirmed with `git check-ignore`).

**Problem:** `profileExecution` passes `--cpu-prof-dir <project>/.moon` (or `--heap-prof-dir`) with
the fixed name `snapshot.cpuprofile` (`toolchains/node/src/tier2.rs:19-41`). **Live**, running a
profiled `node` task three times:

- It wrote `apps/web/.moon/snapshot.cpuprofile`, which `git status` showed as untracked.
- The task's hash changed on every run (`dfc479f5`, `b8d2a36f`, `f169f753`), and so did the hash of a
  `script` task in the same project that isn't profiled, since the profile is an input under the
  default `**/*`. So profiling defeats caching for every task in the project.
- Every `node` task in a project writes the same file, so the last one wins.

moon v1 wrote to `.moon/cache/states/<project>/<task>/` (v1.41.8
`legacy/node/platform/src/actions/run_target.rs:31-33`), which is ignored and is where moon v2 keeps
each task's `stdout.log` and `lastRun.json`.

**Change:** use `context.workspace_root.join(".moon/cache/states/<project id>/<task id>")`, from
`input.project.id` and `input.task.target`. Node creates the directory (live: `apps/web/.moon` didn't
exist beforehand). moon has no input for its cache directory, so hard-coding it is allowed (draft
ADR "virtual paths in WASM, real paths in strings", rule 6); pass the real path, as today. Say where
profiles go in the setting's description.

**Tests:** update the three profile tests in `tier2_test.rs`. Their `expected_prof_dir` comment about
mixed separators on Windows may also be stale once the plugins build against proto 0.64's
`warpgate_pdk`.

**Changelog:** profiles move to `.moon/cache/states/<project>/<task>/`.

## Phase 2: after decisions

### #5 Docker scaffold: copy the files that install scripts run (D1)

- [x] Add to the JavaScript toolchain's `scaffold_docker`, for the configs phase (D1), so every
      package manager gets it (`BUN.md` asked for this)
- [x] Tests: `copies_files_that_install_scripts_run` and the `get_install_script_args` unit test

**Problem:** the only scaffold glob is `*.{js,cjs,mjs}` ("postinstall scripts, etc",
`toolchains/node/src/tier1.rs:69-72`), which moon applies from the workspace root and each project
root (moon `crates/app/src/commands/docker/scaffold.rs:146`, `:170`). **Live:** with
`"postinstall": "node scripts/postinstall.js"` in `apps/web/package.json`, `moon docker scaffold web`
copied `eslint.config.js`, `info.js` and `vite.config.mjs` into the configs skeleton, but not
`scripts/postinstall.js`. `moon docker setup` in the skeleton then failed: npm ran the workspace's
`postinstall`, which exited with `MODULE_NOT_FOUND`. A script in a subdirectory is the usual layout,
and TypeScript ones (`node scripts/setup.ts`, which Node runs natively since 22.18) aren't matched at
all. The glob also copies config files that install never needs, so editing one invalidates the
image's install layer.

**Recommended:** implement `scaffold_docker`. In the configs phase, for the workspace root (no
project) and for each project, read `package.json`'s `scripts` for the lifecycle scripts npm runs
on install: `preinstall`, `install`, `postinstall`, `preprepare`, `prepare` and `postprepare`. Split
each on whitespace and shell operators (`&&`, `||`, `;`, `|`), and copy every token that's a
relative path to an existing file under `input_dir`, outside `node_modules`, unless it was already
copied. Keep the glob, since `node setup` resolves `setup.js` without an extension.

**Alternative:** document `docker.scaffold.configsPhaseGlobs` (moon
`crates/config/src/workspace/docker_config.rs:76`), and change nothing.

**Not covered:** files that those scripts import, and scripts that run a package's bin, like
`patch-package` (which reads `patches/`) or `husky`.

**Tests** (`tier1_test.rs`): a `postinstall` in a subdirectory, a `.ts` script, a chained command,
a bin name (nothing copied), the sources phase (nothing copied), and a root `package.json`.

### #6 A member's `engines.node` beats the root `.nvmrc` (D2)

- [x] Keep `engines.node` (D2). The handbook note is in `MOON_COMPACT.md`.

**Problem:** proto detects a version per directory, nearest first, and the tool reads `engines.node`
as the last fallback in each `package.json` (`tools/node/src/proto.rs:63-100`). Published packages
set `engines.node` to the range they support, not to the version developers use. **Live** (proto
0.64.0): in a repo with `.nvmrc` `22.22.3` at the root, `proto bin node` in `packages/lib`, whose
`package.json` has `"engines": { "node": ">=18" }`, resolved to 26.0.0, the highest installed. moon
tasks only hit this when `node.version` isn't set and `node` resolves to proto's shims, since moon
sets `PROTO_NODE_VERSION` otherwise. fnm reads `engines.node` too (behind `resolve_engines`), as does
`n auto`. mise doesn't, since `engines.node` "describes the Node.js versions a package is compatible
with".

**Recommended:** no change, as fnm and `n` behave the same. Document in the Node.js handbook that
`node.version` or a root `.prototools` pin avoids it.

**Alternative:** stop reading `engines.node`, as mise does. Repos that pin only through `engines`
would then lose detection. Reading it only when no other pin exists up to the root isn't possible,
as proto calls the plugin one directory at a time.

## Phase 3: lower priority

### #7 Detection names

- [x] `config_file_globs` (`toolchains/node/src/tier1.rs:17`): add `.nvmrc` and `.node-version`.

moon uses config globs for project detection and copies them into the configs skeleton (moon
`toolchain_plugin.rs:313`, `:339-375`). Live: `moon docker scaffold` copied neither, so in an image
without `node.version`, proto's shims have no pin. Python #22 added `.python-versions` the same way.

### #8 Default image when `version` is unset (D3)

- [x] Use `node:lts` (an exception in the Docker tags ADR draft)
- [x] Update `handles_image_version`

`define_docker_metadata` falls back to `node:latest` (`tier1.rs:67`), which follows Node's Current
release, and that isn't an LTS for the first months of each major. The images moon generates are for
production builds, and the tool's own `stable` alias already means the latest LTS.

### #9 pnpm's `devEngines.runtime` Node replaces `node.version` in tasks

- [ ] Document it in the Node.js handbook (in `MOON_COMPACT.md`)
- [ ] Consider a warning (left out of the 2026-11-18 pass)

Handed over by `PNPM.md`, which found it **live** with pnpm 10.33.4 and 11.28.12: pnpm installs a
`devEngines.runtime` Node as a dependency, into `node_modules/.bin/node`. The JavaScript toolchain
puts `node_modules/.bin` on PATH ahead of moon's Node (`gather_shared_paths`), so with
`node.version: 24.22.3`, a task printed `v22.22.3` while its hash recorded 24.22.3. That's what
the repo asked pnpm for, so don't override it. Document that `node.version` should match
`devEngines.runtime`, or be left unset. A `debug!` or warning when `node_modules/.bin/node` exists
and `node.version` is set would make it visible, at the cost of a file check per task.

### #10 Parse nodejs.org's index leniently

- [x] Make `files` and `lts` optional, with a catch-all for `lts`, and skip entries that still
      fail (`NodeDistVersion::parse_index`)

Handed over by `JAVASCRIPT.md`, from source: `NodeDistVersion`
(`crates/lang-javascript-common/src/node_dist.rs:3-16`) requires `files` and `lts`, and the
untagged `NodeDistLTS` only accepts a bool or a string. `tools/node` (`proto.rs:185`) and
`tools/node-depman` (`proto.rs:278`) deserialize the whole index into it, so one entry in a new
shape fails version loading for both (ADR-0008.3, .4). Default the fields, add a catch-all
variant, and skip entries that still fail.

## Not planned

- **Syncing `.nvmrc` or `.node-version` from `version`:** plugins#96 was closed by deprecating
  `syncVersionManagerConfig`, and the Ruby and Python plans also chose not to write version files.
  Pinning in `.prototools`, which moon reads with `versionFromPrototools`, gives one source of truth.
- **Hashing `.nvmrc` when `version` is unset:** Ruby #8 removed its equivalent, and Python doesn't
  either. The resolved version is a moon item (see `MOON_COMPACT.md`).
- **`executeArgs` and `profileExecution` for `script` tasks:** documented as task-command only, like
  v1's `binExecArgs`. Passing them through `NODE_OPTIONS` would apply them to every Node process the
  script starts.
- **Aliases that nvm or `n` support and proto doesn't:** `lts/-1`, `system`, `iojs`, `current` and
  mixed-case `lts/Iron` fail with an error (live), not silently. Revisit if users ask.
- **Working around the bundled npm in plugins:** plugins can only add paths, and the `npm` toolchain
  isn't told its own install directory. It needs the moon fix below.
- **`define_requirements` for `javascript`:** moon's docs say the node toolchain requires
  `javascript`, but nothing enforces it, and `node` works on its own. The JavaScript toolchain
  already requires `node`, so this would be a cycle.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added "Order a
toolchain's paths ahead of the toolchains it requires" (the bundled npm), "Hash the resolved
toolchain version", the locator bump in "Ship proto 0.64", and the Node website docs.

**proto:**

- [ ] **Version files in parent directories are skipped outside the home directory.** Since 0.64,
      detection skips files the plugin can't access (`crates/core/src/flow/detect.rs:141-158`, from
      253fbc529), and a working directory outside `$HOME` is mapped as `/cwd` alone
      (`crates/core/src/env.rs:211-236`), so its parents aren't accessible. **Live:** in
      `mono/packages/app` under the scratch directory, `proto bin node` logged "Skipping version
      file, as it's not accessible to the plugin" for `mono/.nvmrc` and fell back to the global pin
      (22.14.0). Docker images (`WORKDIR /app`, `HOME=/root`) and CI runners that check out outside
      the home directory hit this whenever a shim runs below the pinned directory, like a moon task
      in a project when `node.version` isn't set. The tools in this repo read
      `ParseVersionFileInput.path` since f58588fc, so they can't fall back to `content`, which proto
      still sends (`detect.rs:168`). **Change:** map the directory being checked for the call, or
      keep `content` as a fallback for plugins.

## Release

**Blocked**, as for Python: `node_tool` 0.18.0 sets `minimum_proto_version` to 0.64
(`tools/node/src/proto.rs:31`), and moon 2.6.x embeds proto_core 0.62, so a `node_toolchain` built
from `master` can't load until moon ships proto 0.64 (`MOON_COMPACT.md`). Until then, moon users keep
the released 1.0.5 with `node_tool` 0.17.13, which lacks 0.18.0's fixes. **Live:** with an `.nvmrc`
of `lts/*` or `20 # LTS`, `moon toolchain add node` fails with "Failed to parse a version
requirement … expected semver_req".

- `node_toolchain`: ship #2, #3, #7 and #8 as 1.1.0, with changelog callouts for the generated
  Dockerfile's `FROM` and `moon toolchain add` no longer writing `syncVersionManagerConfig`. It
  also picks up `node_tool` 0.18.0, which requires proto 0.64. #4 isn't planned, and #5 is in
  `javascript_toolchain`.
- `node_tool`: #1 and #10 are a patch (0.18.1). They change `lang-javascript-common`, so the Bun,
  Deno and node-depman tools that adopt them need their own releases. The toolchain re-exports
  `node_tool`, so tool fixes also need a toolchain release.

## Handed to other plans

- `NPM.md` (the node-depman tool, shared with `PNPM.md`, `YARN.md` and `NUB.md`):
  `tools/node-depman/src/proto.rs:78` parses the whole `package.json` as `PackageJson` for
  `packageManager` and `devEngines`, so an invalid `version` or dependency specifier drops the pin
  silently (NODE #1 adds a shared struct to switch to).
- `BUN.md`: `tools/bun/src/proto.rs:63` has the same whole-file `PackageJson` parse as NODE #1, and
  `toolchains/bun/src/tier1.rs:59` the same `*.{js,cjs,mjs}` scaffold glob as NODE #5.
- `DENO.md`: `tools/deno/src/proto.rs:58` has the same whole-file `PackageJson` parse as NODE #1, and
  `toolchains/deno/src/tier1.rs:58` the same scaffold glob as NODE #5.
- `PNPM.md`: moon v1 set `NODE_PATH` to `node_modules/.pnpm/node_modules` for `node` tasks under
  pnpm (v1.41.8 `legacy/node/platform/src/actions/run_target.rs:85`); v2 doesn't. Check whether
  anything relied on it.
- `NUB.md`: inferred nub tasks get `[javascript, nub]` without `node`
  (`toolchains/javascript/src/infer_tasks.rs:142-146`), so `node` in their scripts is whatever is on
  PATH unless nub provides it. Confirm that nub puts its Node on PATH for scripts.
