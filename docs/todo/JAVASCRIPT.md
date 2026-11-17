# JavaScript toolchain fix plan

Fixes from the JavaScript toolchain audit (2026-11-17, against moon v2.6.5 source, live-checked with
moon 2.6.4, proto 0.64.0, Node 24.22.3, npm 11.13.0 and 12.2.2, pnpm 10.33.4 and 12.21.0, Yarn
4.18.1 and Bun 1.4.0). Covers `toolchains/javascript` (`javascript_toolchain` 1.3.2, toolchain ID
`javascript`) and `crates/lang-javascript-common` (`lang_javascript_common` 0.1.0, unpublished),
limited to what isn't specific to one runtime or package manager: registration and detection, the
project graph, inferred tasks, `package.json` syncing, the generic dependency-root walk, manifest
hashing, the `parse_lock` dispatch, and the generic Docker flow. Each package manager's installs,
lockfile parser and workspace config, and the Node runtime, are in `NPM.md`, `PNPM.md`, `YARN.md`,
`BUN.md`, `DENO.md`, `NUB.md` and `NODE.md`. Item numbers (#1–#21) match the audit findings, and #22 was found while implementing them.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build.

```bash
cargo build -p javascript_toolchain --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p javascript_toolchain -p lang_javascript_common --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

`infer_tasks.rs` is behind the `wasm` feature (`lib.rs:2-3`) and imports `moon_pdk`
(`infer_tasks.rs:8`), so #2–#6 can only be tested through the WASM sandbox today. To unit test
them, gate the module with `#[cfg(any(feature = "wasm", test))]` and take `AnyResult` from
`moon_pdk_api` (ADR-0006.3, ADR-0002.5).

The `setup_environment` tests mock `load_toolchain_config` with `|_, _|` (`tier2_env_test.rs:18` and
the rest), ignoring the requested ID, and `tier1_sync_test.rs:119` mocks a config that `sync_project`
never loads. New tests for config loading should assert the ID.

Findings marked **Live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in scratch git repos,
against the released `javascript_toolchain` 1.3.2:

- **npm workspaces:** `apps/a` (`"@acme/b": "^1.0.0"`, `"@acme/c": "file:../../packages/c"`, plus
  scripts), `packages/b` (1.2.0), `packages/c` (0.1.0), `packages/d` (0.3.0), `packages/nov` (no
  `version`), and `examples/ex1` and `examples/ex2`, both named `example` and outside the npm
  workspaces.
- **pnpm workspace** with `catalogs.default` and `catalogs.legacy` in `pnpm-workspace.yaml`.
- **Bun repo** migrated from npm, so it has both `bun.lock` and `package-lock.json`.

Since 1.3.2, `master` only changed lockfile and manifest version leniency, none of it in the code
these findings touch. The JavaScript toolchain embeds no proto tool, so unlike the other toolchains
a `master` build loads in moon 2.6.4. Point the toolchain at it to test a fix live:

```yaml title=".moon/toolchains.yml"
javascript:
  plugin: "file://<CARGO_TARGET_DIR>/wasm32-wasip1/release/javascript_toolchain.wasm"
  packageManager: "npm"
  inferTasksFromScripts: true
node: {}
npm: {}
```

With a `master` build, the missing range edge (#15), the missing `prettier`/`preview`/`postcss`
tasks (#5) and `runInCI: false` on the lint task (#3) reproduced the same way.

**Checked, no change needed:**

- `gather_shared_paths` puts the project's, each parent's and then the root's `node_modules/.bin`
  on PATH, the same order `npm run` uses (live). `PNPM.md` handed `NODE.md` the case where pnpm
  installs a `devEngines.runtime` Node into `.bin`.
- `file:` and `link:` edges: `paths_are_equal` normalizes `..`, so `file:../../packages/c` gave
  `c (production)` (live).
- Catalogs in task hashes: moon's hasher calls `locate_dependencies_root` before `parse_manifest`
  (moon `task_hashing.rs:256`, `:294`), so `load_catalogs` finds the cached catalogs. Live:
  `catalog:legacy` was hashed by its integrity. The `default` catalog naming is `PNPM.md` #5.
- A dependency in both `devDependencies` and `peerDependencies` gives moon one edge (peer) (live).
- The `parse_lock` dispatch, and the `lockfiles/mod.rs` helpers, whose unit tests run in the `test`
  task (ADR-0006).
- `define_requirements`, except that it requires `nub` while moon configures `unstable_nub`, which
  makes moon load the plugin twice (`NUB.md` #1, live).
- `load_toolchain_config("nub")` finds `unstable_nub` through moon's stable/unstable fallback (moon
  `crates/config/src/toolchains_config.rs:118`).
- `setup_environment`'s `packageManager` sync runs inside the plugin call, so the setup fingerprint
  skip (plugins#137) doesn't apply to it.
- The configs skeleton holds every project's `package.json` and the root's `package-lock.json`, and
  `moon docker setup` installed from it with npm 11.13.0 (live), except for members that aren't
  projects (#12). npm's production install in `docker prune` removed a nested dev dependency in
  `apps/a/node_modules` itself (live; #13 covers pnpm and Bun).
- Saving `package.json` keeps its key order.

## Decisions

| Decision                                                                    | Outcome                                                               |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| D1 (#15): link a range that a workspace package's version satisfies?        | **Decided:** yes, for npm, Yarn and Bun                               |
| D2 (#16): hash the root `package.json`'s dependencies for every project?    | **Decided:** no, moon v3's inheritable dependencies will cover it     |
| D3 (#17): what may `rootPackageDependenciesOnly` allow in projects?         | **Decided:** local workspace dependencies                             |
| D4 (#18): update synced entries whose range no longer matches?              | **Decided:** yes, for the `version*` formats and peers                |
| D5 (#19): install projects that the root's `workspaces` don't cover?        | **Decided:** no, not in this pass                                     |

#1 is fixed upstream in `nodejs_package_json` instead of with local types (decided 2026-11-17), so
the same fix covers `PNPM.md` #2 and `NODE.md` #1.

## Phase 1: clear-cut fixes

### #1 `package.json` fields the toolchain never reads fail the project graph

**Done (2026-11-17):** fixed upstream instead of with local types. `nodejs_package_json` now
skips any field it can't parse (`lenient`), deserializes a dependency it can't parse as
`VersionProtocol::Unknown`, and accepts `workspace:` ranges (`WorkspaceProtocol::Requirement`), so
this also fixes `PNPM.md` #2 and `NODE.md` #1. Until 0.5.0 is published, the root `Cargo.toml`
patches it to `../../rust-js-ecosystem/crates/package-json`. Tests are in the upstream crate and
`tier2_test.rs` (`loads_manifests_with_fields_it_doesnt_read`).

- [x] Parse `package.json` into our own types
- [x] Skip values that can't be parsed
- [x] Tests
- [x] Changelog

**Problem:** `PackageJson` (`package_json.rs:20`) wraps `nodejs_package_json` 0.4.11's struct, with
the `protocols` feature that the workspace enables for every crate (`Cargo.toml:53`). Several fields
fail the whole file on values npm accepts:

- `version` is a strict `semver::Version` (upstream `src/package_json.rs:26-28`): `1.0`, `""` and
  `v1.0.0` fail.
- `bundleDependencies` is a `Vec<String>` (`:72-73`), but npm documents `true`.
- `engines` is a map (`:85-86`), so the old array form fails.
- Every dependency value is a `VersionProtocol` with `try_from = "String"`
  (`src/protocols/version.rs:75`), so one specifier it rejects fails the file. `PNPM.md` #2 covers
  `workspace:^1` and other ranges, which pnpm links.

`extend_project_graph` loads every project's manifest with `?` (`tier2.rs:35`), so one such file
anywhere fails every moon command. The same load runs in `sync_project` (`tier1.rs:202`),
`locate_dependencies_root` (`tier2.rs:297`), `parse_manifest` (`:790`) and `detect_package_manager`
(`tier1.rs:156`). This breaks ADR-0008.2 (only define the fields the plugin uses) and ADR-0008.5
(skip the entry). **Live:** with each of the first three shapes in `packages/nov/package.json`,
npm 11.13.0 read the workspace (`npm ls`, and `npm install` for `"version": "1.0"`), and every moon
command failed with "Failed to parse JSON file /workspace/packages/nov/package.json", for the
version "unexpected end of input while parsing minor version number".

**Change:** define the fields the toolchain reads in `package_json.rs`, with `#[serde(default)]`:
`name`, `version`, `main`, `module`, `exports` (presence only), `scripts`, the four dependency maps,
`workspaces`, `catalog`, `catalogs` and `packageManager`. Keep `version`, dependency values and
script values as strings, and parse each one where it's used, skipping a failure with `debug!`
naming the file and entry (ADR-0008.6). Saving already re-reads the file as JSON
(`json_config!`'s `save`), so a lean model doesn't lose fields. `NODE.md` #1 adds the same kind of
struct for the tools' version fields, and `PNPM.md` #2 makes the upstream parser accept `workspace:`
ranges.

**Tests** (`package_json_test.rs`, `tier2_test.rs`): a manifest with each shape above still yields
its name and other dependencies, and `extend_project_graph` succeeds with it in the workspace.

**Changelog:** these repos now load instead of failing.

### #2 One unusual script fails the project graph

- [x] Skip scripts whose task ID can't be created
- [x] Infer the task without outputs when the script can't be split
- [x] Tests

**Problem:** with `inferTasksFromScripts`, `infer_from_package_scripts` returns the first error from
any script (`infer_tasks.rs:64-69`), which fails `extend_project_graph` for the whole workspace:

- `create_task_id` (`:74-76`) fails when `Id::clean` leaves nothing. `"//"` keys are the usual way
  to comment `package.json`. **Live:** `"//": "Run build before test"` in one project's `scripts`
  failed every moon command with "Invalid identifier format for ``".
- `shell_words::split(script)?` (`:80`) fails on unbalanced quotes. **Live:**
  `"clean": "echo 'unbalanced"` failed the graph with "missing closing quote". npm only fails when
  the script runs.
- `Output::parse(output_path)?` (`:124`) fails the same way on a value it rejects.

This is ADR-0008.5's case: skip the entry, not the call.

**Change:** skip a script whose ID can't be created, with a `debug!`. When the split fails, infer
the task without output detection. Skip an output that doesn't parse.

**Tests:** a `"//"` script and an unbalanced quote next to valid scripts, which are still inferred.

### #3 Lint tasks with `--max-warnings` never run in CI

- [x] Match watch, help and version flags as whole arguments
- [x] Tests
- [x] Changelog

**Problem:** `has_watch_option` is `script.contains("--watch") || script.contains("-w")`
(`infer_tasks.rs:192-194`), and a match sets `runInCI: false` (`:156-159`). `-w` matches
`--max-warnings`, `--write`, `--workspace`, `--with-deps`, and paths like `build-web.js`.
`--watch` matches `--watchAll=false`, which Create React App users pass to turn watch mode off in
CI. Turborepo's `basic` example lints with `eslint --max-warnings 0` in `apps/web` and
`packages/ui`.
`has_help_or_version_option` (`:182-190`) has the same problem, for example with `--helpers`.
**Live:** `"lint": "echo eslint . --max-warnings 0"` and `"web": "node build-web.js"` were both
inferred with `runInCI: false`, while `build` kept `true`.

**Change:** match the arguments from the `shell_words` split instead of substrings: `--watch`,
`--watchAll` and `-w` for watch mode, and `--help` and `--version`. `-w` is also npm's and pnpm's
workspace flag, so ignore it when the script starts with `npm`, `pnpm`, `yarn` or `bun`.

**Tests:** `eslint . --max-warnings 0`, `prettier --write .`, `node build-web.js`,
`jest --watchAll=false` and `npm run build -w ui` aren't watch tasks; `tsc -w`, `vite build --watch`
and `jest --watchAll` are.

**Changelog:** lint and format tasks that were skipped in CI now run there.

### #4 Inferred npm tasks drop passthrough flags, and npm 12 fails on them

- [x] End npm commands with `--`
- [x] Update tests
- [x] Changelog

**Problem:** inferred tasks run `<package manager> run <name>` (`infer_tasks.rs:97-106`), and moon
appends passthrough args after it. npm treats flags after the script name as its own config unless
they follow `--`. **Live:** `moon run a:args -- --foo bar --watch` with npm 11.13.0 passed only
`["bar"]` to the script, and warned "Unknown cli config "--foo". This will stop working in the next
major version of npm." npm 12.2.2 fails instead: "npm error code EUNKNOWNCONFIG … Unknown cli flags:
--foo --watch". pnpm 10.33.4 and 12.21.0 and Bun 1.4.0 pass the flags on (confirmed); Yarn and nub
weren't checked.

**Change:** for npm, add `--` after the script name. npm 10.0.0, 11.13.0 and 12.2.2 accept a
trailing `--` with nothing after it (confirmed).

**Tests:** update the npm expectations in `infers_package_scripts_when_enabled` (`tier2_test.rs:147`).

**Changelog:** inferred npm tasks' commands change, so their hashes change once.

### #5 `prettier`, `preview` and `postcss` scripts aren't inferred

- [x] Only skip hooks of scripts that exist
- [x] Tests

**Problem:** `is_valid` (`infer_tasks.rs:196-206`) drops every name starting with `pre` or `post`.
npm only runs `pre<name>` and `post<name>` as hooks when `<name>` is a script too. `preview` is
also one of the dev names (`:209`) and in the setting's docs (`config.rs:118-119`), but never gets
there. **Live:** `prettier`, `preview` and `postcss` weren't inferred, and `prebuild` next to
`build` was correctly skipped.

**Change:** skip `pre<x>` and `post<x>` only when `<x>` is in the same `scripts` map, keeping the
`life_cycles` list (`:23`). `DENO.md` #4 drops these filters for `deno.json` tasks, which have no
hooks.

**Tests:** `prettier`, `preview` and `postcss` are inferred; `prebuild` next to `build`, `postinstall`
and `prepare` aren't.

### #6 Root scripts that run moon recurse

- [x] Don't infer scripts that run moon
- [x] Tests

**Problem:** repos that move to moon often keep root scripts like `"check": "moon run :check"` or
`"ci": "moon ci"`. With the root as a project, that becomes `root:check` running `npm run check`,
which runs `moon run :check` and so `root:check` again. **Live:** moon started itself over and over
until it was killed.

**Change:** don't infer a script whose first word, after any `NAME=value` assignments, is `moon` or
`moonx`. Not covered: `npx moon …` and similar wrappers.

**Tests:** `moon run :check` and `CI=1 moon ci` aren't inferred; `moonbeam build` is.

### #7 Synced peer ranges exclude 0.x packages

- [x] Use the minor version for 0.x peers
- [x] Tests

**Problem:** `sync_project_workspace_dependencies` writes peers as `^<major>.0.0`
(`tier1.rs:303-309`), whatever `dependencyVersionFormat` is. For a 0.x package that's `^0.0.0`,
which only matches `0.0.0`. **Live:** a peer on `@acme/d` 0.3.0 was synced as `^0.0.0`, and
`npm install` then failed with "Could not resolve dependency: peer @acme/d@"^0.0.0"" and a 404 from
the registry. With `^0.3.0` it installed.

**Change:** when the major version is 0, write `^<version>`. Keep `^<major>.0.0` otherwise.

**Tests** (`tier1_sync_test.rs`): peers on 0.3.0 and 1.2.0 packages.

### #8 Syncing fails every task in a JavaScript project without a `package.json`

- [x] Skip projects without a `package.json`
- [x] Test

**Problem:** `sync_project` loads the project's `package.json` (`tier1.rs:202-207`), which gives an
empty model when the file is missing, and `save` then reads the file from disk (`json_config!`). A
project gets the toolchain without one when its `language` is `javascript`, or it has an `.npmrc`,
`bunfig.toml` or `deno.json` (`tier1.rs:37-54`). **Live:** `apps/f`, with only a `moon.yml` setting
`language: javascript` and depending on `d`, failed `moon run f:hi` with "Failed to read path
/workspace/apps/f/package.json".

**Change:** return `skipped` when the project has no `package.json`, before the
`rootPackageDependenciesOnly` check.

**Tests** (`tier1_sync_test.rs`): a project without a manifest that depends on one with a manifest.

### #9 Packages without a `version` are never synced

- [x] Only require `version` for the version formats
- [x] Tests

**Problem:** the sync skips a dependency without `version` (`tier1.rs:274-277`). Private workspace
packages often have none, and moon's Node handbook says local-only packages don't have a version to
reference. The default `workspace:*` format, and `star`, `file` and `link`, don't need one.
**Live:** a production dependency on the versionless `@acme/nov` wrote nothing, while `@acme/d`
0.3.0 was synced.

**Change:** require `version` only for the `version`, `version-caret` and `version-tilde` formats.
Write peers on versionless packages as `*`.

**Tests:** a versionless dependency with the default format, and with `version-caret` (skipped).

### #10 Duplicate package names drop a project from the graph

- [x] Key projects by ID
- [x] Tests

**Problem:** `extend_project_graph` collects manifests in a map keyed by package name
(`tier2.rs:50-56`), and iterates that map (`:61`). A later project with the same name replaces the
earlier one, which then gets no alias, no edges, no inferred tasks, and no `input_files` entry.
Example, template and fixture projects often share names, and npm only rejects duplicates among
workspace members. **Live:** with `examples/ex1` and `examples/ex2` both named `example`, `ex1` lost
its `"@acme/b": "*"` edge, which came back when `ex2` was renamed.

**Change:** iterate the projects, and keep a separate name-to-project map for resolving edges, where
the first project wins. Return each project's own alias, since moon skips aliases that conflict
(as `RUBY.md` #14 found).

**Tests:** two projects with the same name both get their edges and tasks.

### #11 A leftover lockfile overrides the real one in task hashes

- [x] Ignore other package managers' lockfiles when the configured one's exists
- [x] Tests

**Problem:** moon parses every lockfile in `lock_file_names` that exists at the root, and merges
them with later files winning per package (moon `task_hashing.rs:269-283`, `:340-343`). The list
runs Bun, Deno, npm, nub, pnpm, Yarn (`tier1.rs:56-71`). Leftovers are common: `bun install`
migrates `package-lock.json` to `bun.lock` and keeps the old file (confirmed with Bun 1.4.0), and a
stray `package-lock.json` or `yarn.lock` appears whenever someone runs another package manager.
**Live:** in a Bun repo with both files, after making `package-lock.json`'s `is-number` integrity
stale, `m:hi`'s hash recorded `sha512-STALEPACKAGELOCK==` instead of `bun.lock`'s integrity.

**Change:** in `parse_lock`, return no dependencies for a lockfile that the configured package
manager doesn't write, when one that it does write exists in `input.root`. `ParseLockInput` has no
toolchain config, so load it with `load_toolchain_config("javascript")`. Each manager's own files
are `bun.lock` and `bun.lockb`, `deno.lock`, `package-lock.json` and `npm-shrinkwrap.json`,
`nub.lock`, `pnpm-lock.yaml`, and `yarn.lock`. Deno 2.9 and nub, which also read other managers'
lockfiles (`tier2.rs:348-370`), then still use a foreign lockfile until their own exists. Check
before reading the file, so a stray `bun.lockb` no longer runs `bun` on every task hash (`BUN.md`
#3).

**Not covered:** the install fingerprint only hashes the first existing lockfile, so the same
leftovers hide installs. That's moon's "Hash every lockfile in the install fingerprint" (see
`MOON_COMPACT.md`).

**Tests:** `bun.lock` with a stale `package-lock.json` (Bun's entries), and `package-lock.json` alone
under Deno (used).

### #12 `docker setup` misses workspace members that aren't moon projects

**Done:** also copies a `file:` tarball, and skips the project's or root's own manifest, which moon
copies.

- [x] Implement `scaffold_docker` for the configs phase
- [x] Copy members' manifests and `file:`/`link:` targets
- [x] Tests

**Problem:** the configs skeleton only gets manifests from project roots and the workspace root
(moon `toolchain_plugin.rs:313-318`). Package managers install every workspace member, and every
local `file:` or `link:` directory. **Live** (from `PNPM.md`): pnpm 10.33.4's `docker setup` failed
with `ERR_PNPM_WORKSPACE_PKG_NOT_FOUND` and `ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND`. **Live:** npm
11.13.0's `npm ci` in the skeleton succeeded but silently didn't link `@acme/tool-x`, a member under
`tools/` that isn't a moon project.

**Change:** add `scaffold_docker`. In the configs phase, when `project` is `None`, glob the root's
members (`extract_workspace_members_and_catalogs`) for `package.json` files and copy them. For
every copied manifest, also copy the `package.json` of each `file:`, `link:` and `portal:`
directory dependency inside the workspace. Patch files are `PNPM.md` #1, which also covers
`NPM.md` #4 and `BUN.md` #4.

**Tests** (`tier1_test.rs`): a member that isn't a project, and a `link:` directory outside projects.

### #13 `docker prune` leaves projects' `node_modules`

- [x] Delete `node_modules` in every pruned project
- [x] Test

**Problem:** `prune_docker` only deletes the root's `node_modules` (`tier1.rs:323-335`). moon v1
also deleted each project's, as "some package managers do not delete stale node modules" (v1.41.8
`crates/app/src/commands/docker/prune.rs:251-264`). **Live** (from `PNPM.md` and `BUN.md`): with
the isolated layouts of pnpm and of Bun's new workspaces, focused projects kept dangling links to
the pruned dev dependencies, and with pnpm unfocused projects kept their whole `node_modules`. npm
11.13.0 cleaned a nested dev dependency itself (live).

**Change:** also delete `node_modules` in each of `input.projects` and `input.project_dependencies`,
as v1 did, and report them in `changed_files`.

**Tests:** a project-level `node_modules` is removed.

### #14 Small fixes

**Done**, except `toolchains/node-depman/src/tier1.rs:65`'s `config_url`, which is `NPM.md` #5.

- [x] `initialize_toolchain`'s `config_url` (`tier1.rs:106`) is
      `https://moonrepo.dev/docs/guides/javascript`, which redirects to a 404 (confirmed with
      curl). Use `…/guides/javascript/node-handbook`, which exists.
      `toolchains/node-depman/src/tier1.rs:65` has the same URL (`NPM.md` #5, `PNPM.md` #7).
- [x] `detect_package_manager` (`tier1.rs:158-162`) only reads `packageManager`. Fall back to the
      first `devEngines.packageManager` entry's name, which `pnpm init` writes since pnpm 11 and
      where nub records itself (`PNPM.md`, `NUB.md`). Skip a name it doesn't know instead of
      failing `moon init` with `from_str`'s error (`:161`).
- [x] Inferred task IDs collide: `build:js` and `build-js` both become `build-js`
      (`infer_tasks.rs:66-67`), and one replaces the other. **Live:** `build-js` ran
      `npm run build:js`. Keep the first, with a `debug!`.
- [x] `version_satisfies` (`config.rs:153-175`) returns `false` for aliases, so `version: latest`
      turns off every version-gated flag, like `bun ci` (`DENO.md`). Return `true` for `latest` and
      `stable`, as for `canary`. An unset version stays `false`, as nothing says which one runs.
      It's also `false` for every prerelease, as `>=2.0.0` doesn't match `6.0.0-rc.23` under
      semver, so prereleases take the old-version paths (`YARN.md`). `YARN.md` #3 replaces the Yarn
      uses with a major-version helper; check the Bun and Deno gates against a prerelease too.
- [x] `host_log!` markup `<id>{}</file>` (`tier1.rs:313`).
- [x] `parse_lock`'s error (`tier2.rs:693`) names the virtual path, like
      `Failed to parse /workspace/bun.lockb` (`BUN.md`). Use the real path.
- [x] `lang-javascript-common`: `extract_volta_version` (`package_json.rs:88-115`) follows `extends`
      with no limit, so a cycle overflows the stack, which is a trap that the panic hook can't
      report. Stop after a few levels. `extract_package_manager_version` (`:64-86`) splits on every
      `@`, so Corepack's URL form (`yarn@https://registry.npmjs.org/@yarnpkg/cli-dist/…`) gives
      `https://registry.npmjs.org/`; split once.
- [x] Make the `load_toolchain_config` mocks in `tier2_test.rs` and `tier2_env_test.rs` assert the
      requested ID (`NPM.md`, `DENO.md`, `NUB.md`).

### #22 Syncing rewrites the other dependencies in a field

- [x] Write only the entries the sync adds or changes
- [x] Tests

Found while implementing #18. `PackageJson::save_field` wrote every entry of a dirty dependency
field back through `VersionProtocol`'s `Display` (`package_json.rs:36-45`), so adding one workspace
dependency re-formatted the rest. `nodejs_package_json` normalizes ranges into Cargo's syntax:
`>=1.0.0 <2.0.0` becomes `>=1.0.0, <2.0.0`, and `18.x` becomes `~18`. npm's own `semver` rejects the
comma form (`validRange` returns `null`), so the next install fails.

**Change:** dependencies are marked dirty one at a time (`dependencies:<name>`), and `save_field`
only writes those entries. Test: `keeps_the_text_of_entries_it_doesnt_change`.

## Phase 2: after decisions

### #15 Ranges to workspace packages aren't edges (D1)

- [x] Link ranges that the workspace package's version satisfies, for npm, Yarn and Bun
- [x] Tests
- [x] Changelog

**Problem:** `extend_project_graph` only treats `workspace:`, `file:`, `link:` and `portal:` to the
package's directory, and `*`, as local (`tier2.rs:78-89`). npm, Yarn (whose
`enableTransparentWorkspaces` is on by default) and Bun link a workspace package whenever its version
satisfies the range. **Live:** with `"@acme/b": "^1.0.0"` and a local 1.2.0, npm 11.13.0, Yarn
4.18.1 and Bun 1.4.0 all symlinked the workspace package, and pnpm 10.33.4 (with
`link-workspace-packages` off by default) went to the registry. moon showed no `a → b` edge, so:

- `^:` task dependencies and affected detection miss b, and so does `moon docker scaffold a`.
  **Live:** `.moon/docker/sources` had `packages/c` (a `file:` dependency) but not `packages/b`.
- The sync's own `version`, `version-caret` and `version-tilde` formats (`config.rs:46-57`) write
  ranges that the graph then doesn't read back.

`NPM.md` #7 is the npm part of this, `BUN.md` saw the same with Bun 1.2.20 and 1.4.4, and
`YARN.md` #12 adds Yarn 1, which has no `workspace:` protocol, so its workspaces depend on each
other by exact version (**live** with Yarn 1.22.22: `"b": "1.0.0"` gave no edge).

**Recommended:** for npm, Yarn and Bun, also treat `Requirement`, `Range` and `Version` specifiers
as local when the workspace package's `version` satisfies them. Leave pnpm, Deno and nub as they are;
pnpm only links ranges with `link-workspace-packages`, which isn't read.

**Alternative:** document that edges need `workspace:`, `*` or a path.

**Risk:** new edges can expose cycles, like two packages that list each other as dev dependencies.
That needs a changelog callout.

**Tests:** a matching range (edge), a range the version doesn't satisfy (no edge), an exact version,
and pnpm (no edge).

### #16 The root `package.json`'s dependencies aren't hashed (D2)

**Not planned (D2):** moon v3's inheritable dependencies will cover it.

- [ ] Add the root's dependencies to each project's manifest output
- [ ] Tests
- [ ] Changelog

**Problem:** moon hashes the dependencies in the project's own manifest, and only reads the
workspace manifest for entries marked inherited (moon `task_hashing.rs:425-431`). `parse_manifest`
never marks any, and neither the root `package.json` nor the lockfile is a task input by default.
moon's Node handbook recommends installing tools like TypeScript and ESLint at the root. moon v1
hashed the root `package.json`'s dependencies for every task (v1.41.8
`legacy/node/platform/src/actions/run_target.rs:146-150`), so this is a regression. **Live:** after
`npm install -D is-number@6.0.0` at the root (was 7.0.0), `b:typecheck`, which prints `is-number`'s
version, was a cache hit with the same hash and replayed `7.0.0`.

**Recommended:** in `parse_manifest`, when `input.path` isn't `<input.root>/package.json`, also
report the root's `dependencies` and `devDependencies` that the project doesn't declare, resolved
with `create_manifest_dependency` (catalogs included), in the same fields. Resolved versions work
without moon's inherited handling (moonrepo/moon#2799). The install fingerprint's set of versions
doesn't change, as the root is already in it.

**Cost:** each root dependency bump invalidates every JavaScript task, as in v1.

**Tests:** a project gets the root's dependencies, and its own declaration wins over the root's.

**Changelog:** this changes every JavaScript task hash once.

### #17 `rootPackageDependenciesOnly` rejects what the sync writes (D3)

- [x] Allow local workspace dependencies
- [x] Tests

**Problem:** the check fails when a project's `package.json` has any dependency field, even empty
(`tier1.rs:209-220`), and the sync below it, on by default, adds workspace dependencies to that same
file. **Live:** with `rootPackageDependenciesOnly: true`, the first `moon sync projects` added
`"@acme/d": "file:../../packages/d"` to `apps/e`, and from then on every task in `e` failed with
"Dependencies can only be defined in the root package.json, found dependencies in project e."
moon v1's `rootPackageOnly`, which this replaced, put every project in the root's install and never
failed (v1.41.8 `legacy/node/platform/src/node_platform.rs:141-144`).

**Recommended:** only fail on entries that aren't local workspace packages (the same rule as the
graph's, #15), and ignore empty maps.

**Alternative:** skip the workspace dependency sync when the setting is on. pnpm needs those entries
to link workspace packages, though.

**Tests:** synced workspace dependencies pass, and an external dependency still fails.

### #18 Synced version ranges are never updated (D4)

- [x] Replace stale ranges for the version formats
- [x] Tests

**Problem:** the sync only adds missing entries (`if_missing: true`, `tier1.rs:298-308`), so with
`version`, `version-caret` and `version-tilde` the written range goes stale on the next bump.
moon's Node handbook says the versions "will also be automatically synced when changed". **Live:**
with `version-caret`, `apps/e` got `"@acme/d": "^0.3.0"`. After bumping `d` to 0.4.0,
`moon sync projects` left it, and npm then fetches `@acme/d@^0.3.0` from the registry, as in #7.

**Recommended:** for the version formats and peers, replace an existing entry when the workspace
package's version no longer satisfies it. Leave other entries alone, so hand-written `workspace:`
ranges and paths stay.

**Tests:** a stale range is replaced, a matching one and a `workspace:` entry aren't.

### #19 Projects that the root's `workspaces` don't cover get no install (D5)

**Not planned (D5).**

- [ ] Projects with their own `package.json` become their own root
- [ ] Roots without `workspaces` report `**/*` members
- [ ] Tests

**Problem:** `locate_dependencies_root` returns the nearest root with a lockfile or `workspaces`
(`tier2.rs:372-391`), whether or not the project is a member. moon then finds no role for the
project (moon `toolchain_plugin.rs:164`), and adds no `InstallDependencies` or `SetupEnvironment`
(moon `action_graph_builder.rs:421`). Two layouts hit this:

- A project with its own `package.json` outside the root's `workspaces`, which npm and Bun treat
  as a separate project. **Live:** `moon run ex1:hi` added only `SetupToolchain` actions, so its
  dependencies are never installed and no lockfile is ever created for it. `BUN.md` saw the same
  with Bun 1.4.0, where `bun install` in that directory makes its own `bun.lock`.
- A project without a `package.json` under a root without `workspaces` (`members: None`), which uses
  the root's dependencies. **Live** (from `NPM.md`): `moon run docs:run` failed with "Cannot find
  module 'ms'" on a fresh clone.

**Recommended:**

- When the starting directory has its own `package.json` and the root's members don't match it
  (`starbase_utils::glob`), return the starting directory as a root with no members.
- When the root has no `workspaces`, report `members: ["**/*"]`, as Go #14 did. The root's role
  then becomes a workspace root, so its commands run without a project, which still gets every
  toolchain's env since moon v2.5.0.

**Cost:** moon globs `**/*` for manifests when one changes (moon `install_dependencies.rs:333`),
skipping `node_modules`.

**Tests:** a non-member project with a `package.json` is its own root, and a project without one
under a root without `workspaces` is a member.

## Phase 3: lower priority

### #20 `npm:` aliases and dist tags aren't hashed

- [x] Report an alias's range
- [x] Report tags as `*`
- [x] Tests

**Problem:** `create_manifest_dependency` returns nothing for `npm:` and `jsr:` aliases
(`tier2.rs:719-725`), so they reach neither task hashes nor the install fingerprint. **Live** (from
`PNPM.md`): `"string-width-cjs": "npm:string-width@^4.2.0"` never reached task hashes. Dist tags
(`"latest"`, `"next"`) become `UnresolvedVersionSpec::Alias`, which never matches a locked version
(moon `task_hashing.rs:407-411`), so moon hashes the word `latest`, and lockfile changes don't change
the hash. Found from source.

**Change:** for aliases, report the range after the last `@` under the alias name. moon then
records the locked entry where the lockfile keys it by alias (npm, per `NPM.md`), and the range
otherwise. For tags, report `*`, so moon records the first locked entry.

**Tests:** an alias with a range, and a `latest` tag.

### #21 Cache writes shouldn't fail the call

**Done:** only the log-and-continue half. moon's PDK is still on `warpgate_api` 0.19.

- [x] Log and continue on `var::set` errors

`extract_workspace_members_and_catalogs` returns `var::set` errors (`tier2.rs:309-315`), so
`locate_dependencies_root` fails when the store is full. moon 2.6 keeps Extism's 1 MiB var limit, so
only very large catalogs are at risk. The draft caching ADR (rule 7) says to log and continue.
Switch the keys to `create_var_key` once `warpgate_api` 0.21 ships (rule 5).

## Not planned

- **npm's lifecycle env for tasks that don't use `<pm> run`:** tasks that run `vite build` directly
  don't get `npm_package_*` or `npm_lifecycle_event`. Inferred tasks go through `npm run`, so they
  do.
- **Inferring `pre`/`post` hooks as task deps:** the package manager runs them inside `run`, where
  it supports hooks.
- **`optionalDependencies` as `Build` edges** (`tier2.rs:110`): `Build` is exempt from layer
  constraints and isn't synced, which suits optional packages. No one has reported a problem.
- **`overrides` and `resolutions` in the install fingerprint** (from `NPM.md`): the install re-runs
  once the lockfile changes. Hashing whole manifests would reinstall on every script edit.
- **Keeping indentation without an `.editorconfig`:** saves use 2 spaces when there's none, as
  `starbase_utils` does for every plugin.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added dependencies
without a version to "Fingerprint everything that changes an install", "Store the install
fingerprint after the install", the alias case in "Focus `docker prune` installs by package name,
without the root", evidence for "Hash every lockfile in the install fingerprint", and the JavaScript
website docs.

## Release

- **Before merging:** publish `nodejs_package_json` 0.5.0 (the new `WorkspaceProtocol::Requirement`
  variant is breaking), bump the workspace dependency to it, and remove the `[patch.crates-io]`
  entry from the root `Cargo.toml`.
- `javascript_toolchain` embeds no proto tool, so it isn't blocked by moon's proto 0.64 release (a
  `master` build loads in moon 2.6.4). Phase 1 changes which tasks are inferred and how (#3–#6),
  what the sync writes (#7, #9), task hashes (#4, #11) and Docker output (#12, #13), so ship it as
  1.4.0 with changelog callouts. Phase 2 changes graph edges (#15) and every task hash (#16), so it
  can follow as 1.5.0. Phase 3 is a patch.
- `lang_javascript_common` is unpublished. The node, node-depman, Bun and Deno tools pick up #14's
  `volta` and `packageManager` fixes in their next releases, which `NODE.md` and the package manager
  plans schedule.

## Handed to other plans

- `NODE.md`: `crates/lang-javascript-common/src/node_dist.rs:3-16` types nodejs.org's index with an
  untagged `NodeDistLTS` that has no catch-all, and requires `files` and `lts` (ADR-0008.3, .4).
  Found from source.
- `YARN.md`: Yarn PnP creates no `node_modules`, so `vendor_dir_name` (`tier1.rs:97`) makes every
  run a first install, and the `node_modules/.bin` entries that `gather_shared_paths` adds don't
  exist. Found from source.
- `YARN.md`: `get_supported_for` (`config.rs:94`) keeps `workspace:*` for Yarn 1, which has no
  `workspace:` protocol. Found from source.
- `YARN.md`, `BUN.md`: #15 covers range edges for both (live: Yarn 4.18.1 and Bun 1.4.0 linked
  `^1.0.0` to a local 1.2.0). Yarn with `enableTransparentWorkspaces: false` isn't covered.
- `PNPM.md`: #15 leaves pnpm out because `link-workspace-packages` isn't read; reading it would let
  pnpm repos that turn it on get range edges.
- Already in other plans and confirmed here: `PNPM.md` #2 (`workspace:^1.0` failed the graph),
  `PNPM.md` #5 (`catalog:` against `catalogs.default` dropped `is-number` from a hash), `PNPM.md` #9
  (pnpm 12.21.0 followed `devEngines.packageManager` over `packageManager`), `NODE.md` #1 (proto
  0.64.0 resolved pnpm 12.21.0 instead of `packageManager: pnpm@10.18.0` once `"version": "1.0"`
  was added), `NPM.md` #1 (nested `package-lock.json` entries never match: `a:build` hashed the range
  `6.0.0`), `NPM.md` #5 (`--production` warns "Use `--omit=dev` instead"), and `DENO.md` #4 (the
  `pre`/`post` filter).
