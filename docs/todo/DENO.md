# Deno toolchain fix plan

Fixes from the Deno toolchain audit (2026-11-17). Checked against moon v2.6.5 source, and live with
moon 2.6.4, proto 0.64.0 and Deno 2.9.8 (the current release), plus Deno 2.0.6, 2.1.0, 2.7.14 and
2.8.0 where behavior depends on the version. Covers:

- `toolchains/deno` (`deno_toolchain` 1.1.3, toolchain ID `deno`): registration, config, Docker
  metadata, task command hooks, `parse_manifest` and `setup_environment`.
- `tools/deno` (`deno_tool` 0.16.0), which the toolchain re-exports as tier 3. It was audited on its
  own on 2026-11-06, so it's only covered where the toolchain depends on it.
- The Deno parts of `toolchains/javascript` (`javascript_toolchain` 1.3.2, toolchain ID
  `javascript`), which does Deno's tier 2 work: installs, the dependencies root, workspace members,
  `deno.lock` parsing and task inference.

Item numbers (#1–#15) match the audit findings.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build. Build tools and toolchains in separate
cargo runs.

```bash
cargo build -p deno_tool --target wasm32-wasip1 --release
cargo build -p deno_toolchain -p javascript_toolchain --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p deno_toolchain -p deno_tool -p javascript_toolchain --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

Findings marked **Live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in scratch git
repos, against the released `deno_toolchain` 1.1.3 and `javascript_toolchain` 1.3.2 that moon
downloads. Their Deno code paths match `master`, except the ADR-0008.5 version skips and the
`DENO_INSTALL_ROOT` fallback in #3. A `master` build can't load in moon 2.6.x, since the embedded
tool requires proto 0.64 (see "Release"). The repos:

- **Workspace:** a root `deno.json` with `"workspace": ["./apps/app", "./packages/lib"]`. `app` has
  a `deno.json` that imports `jsr:@std/path@^1.0.0` and `npm:chalk@^5.3.0`, a `main.ts` that also
  imports the `@x/lib` member by name, and a deno.json `start` task. `lib` is named `@x/lib` and
  exports `./mod.ts`. There's no `package.json` anywhere.
- **Package:** a `web` project with a `package.json` (depending on `is-odd`) and a `deno.json` with
  `build` and `start` tasks, where `start` depends on `build`.
- **Mixed:** a root `deno.json` workspace with member `a`, and a root `package.json` whose
  `workspaces` lists `b`.

The toolchains were configured as `deno: {}` (or with `version`) and
`javascript: { packageManager: deno }`. Isolate Deno state with scratch `PROTO_HOME`, `DENO_DIR` and
`DENO_INSTALL_ROOT`, and drop `~/.deno/bin` from `PATH`, or PATH checks see the shell's own entry.

Test mocks: the Deno install tests in `toolchains/javascript/tests/tier2_test.rs` mock
`load_toolchain_config` with `|_, _|`, which ignores the requested ID. New Deno tests should assert
`deno`. Several Deno toolchain tests call `create_toolchain("rust")` (#9).

**Checked, no change needed:**

- `deno.lock` parsing: deno_lockfile 0.51.0 parses v4 (written by Deno 2.0.6 and 2.1.0) and v5
  (written by 2.7.14 and later; 2.9.8 also rewrites v4 files as v5), including `os` and `cpu`,
  `optionalDependencies`, the `scripts`, `bin` and `deprecated` flags, peer suffixes, and the
  `remote` and `workspace` sections. It ignores unknown fields. The latest deno_lockfile, 0.62.0,
  still tops out at v5. Only v3 (Deno 1.x) returns nothing (#9).
- `deno.jsonc`: `starbase_utils::json::read_file` strips comments.
- Workspace members written with `./` (`"./apps/app"`), as Deno's docs do, match moon's member
  globs. Live, the member got the root's `InstallDependencies(javascript)`.
- Ignoring `input.packages` for focused `docker prune` installs is right: `deno install` has no
  filter flag. Deno also has no dedupe command.
- `get_supported_for` passes every format through for Deno. Deno 2.9.8 accepts `workspace:*`,
  `workspace:^`, `workspace:~`, `file:`, `link:`, `*` and versions between members. Deno 2.0.6 only
  accepts `workspace:*` and versions, but `workspace:*` is the default.
- Version pins: `moon toolchain add deno` detects the version through the tool's
  `detect_version_files` (moon `crates/app/src/commands/toolchain/add.rs:88`), so it reads `.dvmrc`
  and `package.json` the same way proto does. `.deno-version` isn't a convention; setup-deno reads
  `.dvmrc` and `.tool-versions`.
- The `--prod` gate (2.8.0+) and the `deno ci` gate are right: Deno 2.7.14 has neither.
- Graph cache: `deno.json` edits invalidate it, as moon hashes every toolchain's manifest names in
  each project (moon `crates/workspace/src/workspace_cache.rs:233-241`).
- `deno install --global` run from the dependencies root doesn't touch that root's `deno.lock`
  (confirmed). The bin commands use `CacheStrategy::Memory`, so moon runs identical ones once per
  process, under a lock (moon `crates/actions/src/plugins/commands.rs:82-99`).
- `DENO_INSTALL_ROOT` ending in `bin` is used as-is by Deno 2.9.8 (confirmed), as `append_bin_dir`
  assumes.

## Decisions needed

| Decision                                                                  | Recommendation                                                                     |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| D1 (#10, #12): where should Deno's setup and `deno.json` parsing run?     | The JavaScript toolchain, which already owns Deno's root and lockfile              |
| D2 (#13): should projects with only a `deno.json` join the project graph? | Yes: alias from `name`, inferred tasks, and edges from `imports` naming a project  |

D1 applies to every toolchain whose tier 2 lives in another toolchain. Node hit it in plugins#96,
which was closed by deprecating `syncVersionManagerConfig` rather than moving it, so there's no
precedent either way, and whichever way it's decided may be worth an ADR.

## Phase 1: clear-cut fixes

### #1 CI installs: `deno ci` rejects `installArgs`, and unversioned configs never freeze

- [ ] Use `deno install --frozen` in CI when `deno.lock` exists, for every version
- [ ] Keep the 2.8.0 gate for `--prod` only
- [ ] Tests

**Problem:** the Deno arm of `install_dependencies` (`toolchains/javascript/src/tier2.rs:463-489`)
runs `deno ci` in CI when a `deno.lock` exists and `deno.version` satisfies `>=2.8.0` (`:469`):

- `installArgs` are then appended to `deno ci` (`:585-589`), which only accepts `--prod`,
  `--skip-types` and `--env-file`. **Live** (`version: '2.9.8'`,
  `installArgs: ['--allow-scripts']`, `CI=true`): the install ran `deno ci --allow-scripts` and
  failed with "unexpected argument '--allow-scripts' found". `--allow-scripts` is how Deno runs npm
  lifecycle scripts (esbuild, sharp), so it's a likely install arg. 2.8.0 rejects it too.
- `version_satisfies` returns `false` when `version` is unset or an alias like `latest`
  (`toolchains/javascript/src/config.rs:153-156`, `:173`), which is how most Deno repos configure
  it. **Live** (`deno: {}`, `CI=true`): CI ran a plain `deno install`. That silently rewrites a
  stale `deno.lock`: with an import added to `deno.json`, Deno 2.9.8 and 2.0.6 both exited 0 and
  added it to the lockfile.
- A production install in CI runs `deno ci` without `--prod` (`:479`), though `deno ci --prod`
  exists since 2.8.0. (`--prod` doesn't drop dev dependencies when a lockfile exists anyway; see
  "Outside this repo".)

**Change:** when `env.ci` is set and `<root>/deno.lock` exists, run `deno install --frozen`, with
`--prod` (2.8.0+) and `installArgs` as usual, and drop `deno ci`. Confirmed:

- `--frozen` works on Deno 2.0.6, 2.7.14, 2.8.0 and 2.9.8. 2.9 documents it as `--frozen-lockfile`
  and still accepts `--frozen`.
- A stale lock fails with "The lockfile is out of date. Run `deno install --frozen=false`…".
- Passing it twice is accepted, so a user's own `--frozen` is harmless.

`deno ci` only adds deleting `node_modules` first, which moon's CI skip makes moot (moon
`crates/actions/src/actions/install_dependencies.rs:126`).

**Alternative:** keep `deno ci` when `installArgs` is empty and a 2.8.0+ version is set. That keeps
two code paths for the same result.

**Not covered:** a custom lock path (`"lock": "other.lock"` or `{ "path": … }` in `deno.json`).

**Tests** (`tier2_test.rs` `mod deno`): CI with a lock and no version, CI without a lock, CI with
`installArgs`, and CI with `production` on 2.8.0 and 2.7.0. Update `switches_to_ci_in_ci` and
`skips_ci_on_older_versions`, with mocks that assert the `deno` ID.

### #2 `executeArgs` break every `deno <subcommand>` task

- [ ] Insert the args after the subcommand
- [ ] Tests

**Problem:** `extend_task_command` prepends `executeArgs` to the task's args
(`toolchains/deno/src/tier2.rs:63-65`), so `deno run -A main.ts` becomes
`deno <executeArgs> run -A main.ts`. Deno only accepts global flags like `--quiet` before the
subcommand. **Live** (`executeArgs: ['--allow-env']`): moon ran `deno --allow-env run -A main.ts`,
which failed with "unexpected argument '--allow-env' found". Deno 2.0.6 fails the same way ("tip:
'run --allow-all' exists"). Inferred tasks (`deno task <name>`) break too.

**Change:** when the first arg is a subcommand that takes runtime flags (`run`, `test`, `bench`,
`serve`, `eval`, `repl`, `compile` and `x`, per `--help` in 2.9.8), return
`Extend::Replace` with the args inserted after it. When the first arg is a script
(`deno main.ts`), keep prepending, which Deno accepts. For other subcommands (`task`, `fmt`, `lint`,
`check`, `install`), add nothing; `deno task --allow-env` fails too.

**Tests:** `deno run`, `deno test`, `deno main.ts`, `deno task build` (unchanged) and `deno -q run`.

### #3 Deno's bin directory isn't on `PATH` without `version`

- [ ] Always return the directory `deno install --global` writes to
- [ ] Tests
- [ ] Changelog

**Problem:** `gather_shared_paths` returns no paths when `globals_dir` resolves
(`toolchains/deno/src/tier2.rs:20-26`). moon always passes one (moon
`crates/toolchain-plugin/src/toolchain_plugin.rs:450-453`), as proto falls back to the last lookup
directory, `$HOME/.deno/bin`, and creates it (`proto_core` 0.62.2 `flow/locate.rs:395-410`). moon
only adds globals directories itself when `version` is set (`toolchain_plugin.rs:228-238`). This is
Go #5.
**Live** (clean `PATH`): with `deno: {}`, the deno plugin returned `{}` and a task had no Deno bin
directory on `PATH`; with `version: '2.9.8'`, moon added `~/.deno/bin`.

proto's fallback is also wrong when `DENO_INSTALL_ROOT` is set: **live**, with an empty
`$DENO_INSTALL_ROOT/bin`, moon passed `globals_dir: /userhome/.deno/bin`, while `deno install -g`
writes to `$DENO_INSTALL_ROOT/bin`.

The Unreleased changelog entry for `DENO_INSTALL_ROOT` ending in `bin` only changed the branch below
the early return, so it doesn't apply under moon yet.

**Change:** push one directory, without the early return:

- `append_bin_dir($DENO_INSTALL_ROOT)` when it's set (`get_host_env_var`, ADR-0004.1)
- otherwise `globals_dir`
- otherwise `<home>/.deno/bin`

Drop `DENO_HOME` (#9).

**Check first:** `$HOME/.deno/bin` is also where Deno's install script puts `deno` itself. moon puts
a toolchain's globals directories ahead of its executable directory, so with `deno.version` set,
an installer Deno there may replace the configured one, as `BUN.md` #1 reproduced for
`~/.bun/bin` (handed over by `BUN.md`; not checked for Deno). If it does, add `BUN.md` #1's
`activate_environment`, returning the tool directory, to `tools/deno` too.

**Tests:** replace `doesnt_inherit_paths_if_globals_dir` (`tier2_test.rs:53-66`) with a case that
returns `globals_dir`, and one where `DENO_INSTALL_ROOT` wins over it, for task commands and
scripts.

**Changelog:** reword the Unreleased entry to cover both fixes.

### #4 Inferred Deno tasks run their dependencies twice, and lose tasks named like npm hooks

- [ ] Use `deno task --eval` for tasks with `dependencies`
- [ ] Skip the npm lifecycle filters for `deno.json` tasks
- [ ] Infer dependency-only tasks as `noop`
- [ ] Tests

**Problem:** `infer_from_deno_tasks` (`toolchains/javascript/src/infer_tasks.rs:30-58`) turns each
task into `deno task <name>` (`:91-95`), with its `dependencies` as moon `deps`. Deno runs a task's
`dependencies` itself, and has no flag to skip them. **Live:** `moon run web:start` ran `build`
twice, once as `web:build` and again inside `deno task start` (`build.log` got two lines).

The inferrer also applies npm's rules through `is_valid` (`infer_tasks.rs:196-206`): lifecycle
names and anything starting with `pre` or `post` are dropped, and so are tasks with no command.
Deno has no lifecycle hooks for `deno.json` tasks (confirmed: `deno task build` doesn't run
`prebuild`). **Live:** `deno.json` tasks named `preview`, `prebuild` and `install`, and a task with
only `dependencies`, weren't inferred.

**Change:**

- For a task with `dependencies`, infer `deno task --eval "<command>"` and keep the moon `deps`.
  `--eval` runs the command through Deno's task shell without the task's dependencies. It exists in
  2.1.0, the first release with `dependencies`, through 2.9.8 (2.0.6 has neither).
- Don't apply the lifecycle and `pre`/`post` filters to `deno.json` tasks. Keep them for
  `package.json` scripts, which `deno task` does run with their `pre` scripts (confirmed).
- Infer a task with no command but with `dependencies` (2.9.8 accepts these, 2.1.0 doesn't) as
  `command: noop` with its `deps`.

**Tests:** extend `infers_deno_tasks_when_enabled` with a `dependencies` task, a `preview` task and
a dependency-only task.

### #5 `deno.lock` entries don't match `package.json` names, and names with `_` are dropped

- [ ] Key npm entries by package name
- [ ] Strip the peer suffix after splitting the name
- [ ] Unit tests (ADR-0006)
- [ ] Changelog

**Problem:** `parse_deno_lock` keys npm packages as `npm:<name>`
(`toolchains/javascript/src/lockfiles/deno.rs:36`). moon looks locked versions up by the manifest's
dependency name (moon `crates/task-hasher/src/task_hashing.rs:439`), and `package.json` uses the
bare name, as every other lockfile parser here does. **Live** (the package repo): the `web:build`
hash recorded `is-odd: ^3.0.1`, the requirement, instead of the locked integrity, so a lockfile bump
within the range never changes the hash.

The name is also split wrong. `parse_name_and_version(&key, "_")` (`deno.rs:30`) cuts the id at the
first `_` before finding the version (`lockfiles/mod.rs:41-48`). Deno's ids are
`<name>@<version>[_<peers>]`, and npm names can contain `_`. Run on real 2.9.8 keys,
`string_decoder@1.3.0`, which `readable-stream` pulls in, returns `None`, so it's dropped, while
`ts-node@10.9.2_@types+node@26.6.6_typescript@7.0.2` parses.

**Change:**

- Key npm entries by the bare name. Keep `jsr:<name>` for JSR, where names can overlap npm scopes;
  #12 keys `deno.json` imports the same way.
- In `parse_name_and_version`, find the version's `@` first, then cut the version at the delimiter.
  That gives the same results for pnpm's `(` and Bun's empty delimiter, which the existing tests
  cover.

**Tests:** in `lockfiles/mod.rs`, which runs outside the `wasm` feature: `string_decoder@1.3.0` and
the `ts-node` peer key. Update `parses_deno` for the new keys.

**Changelog:** Deno task hashes change once for projects with `package.json` dependencies.

### #6 Members from a root `package.json` are ignored when `deno.json` has a `workspace`

- [ ] Merge both member lists
- [ ] Take catalogs from `package.json` when both define them
- [ ] Tests

**Problem:** for Deno, `extract_workspace_members_and_catalogs` takes members from `deno.json`
(`toolchains/javascript/src/tier2.rs:265-270`), and only reads `package.json` when there are none
(`:296-306`). Deno uses both: with `"workspace": ["./a"]` in `deno.json` and `"workspaces": ["b"]`
in `package.json`, Deno 2.9.8's lockfile lists `a` and `b` as members. **Live** (the mixed repo):
`moon run a:x` added `SetupEnvironment(javascript)` and `InstallDependencies(javascript)`, and
`moon run b:x` added neither, so `b`'s tasks run without an install on a fresh clone.

Catalogs follow the opposite rule in Deno: when both root files define catalogs, `package.json`
wins entirely (Deno's workspace docs). The plugin prefers `deno.json`'s.

**Change:** for Deno, read both files and concatenate the member lists, deduplicated. Take
`package.json`'s catalogs when it defines any, otherwise `deno.json`'s.

**Tests:** `locate_dependencies_root` with both member lists, and catalogs in both files.

### #7 Docker default image for partial versions and aliases

- [ ] Only use full versions
- [ ] Tests

**Problem:** `define_docker_metadata` formats `denoland/deno:{to_partial_string()}`
(`toolchains/deno/src/tier1.rs:49-57`). Docker Hub only publishes full versions: `2.9.8`, `2.9.0`
and `latest` exist, while `2.9`, `2.0`, `2` and `canary` return 404. **Live:** with
`version: '2.9'`, `moon docker file app --defaults` wrote `FROM denoland/deno:2.9`. The test
asserts `denoland/deno:1.2` (`tests/tier1_test.rs:25-34`), which doesn't exist either.

**Change:** use the version when it has major, minor and patch (`2.9.8`, or `^2.9.0` as `2.9.0`),
and `latest` otherwise, including aliases.

**Tests:** `2.9.8`, `^2.9.0`, `2.9`, `~2.9`, `latest`, `canary` and unset.

### #8 Docker scaffold copies project sources into the configs skeleton

- [ ] Drop `ts` from the scaffold glob
- [ ] Test

**Problem:** `scaffold_globs` is `*.{js,cjs,mjs,ts}` (`toolchains/deno/src/tier1.rs:58-61`), for
"postinstall scripts". moon applies it in every project root. `main.ts` and `mod.ts` at a project
root are Deno's convention, so every source edit invalidates the image's dependency layer. **Live:**
`moon docker scaffold app` put `apps/app/main.ts` and `packages/lib/mod.ts` into
`.moon/docker/configs`.

The install doesn't need them. In that configs skeleton with every `.ts` file removed,
`deno install --frozen` succeeded with an empty `DENO_DIR`. Deno only runs a root package's
lifecycle scripts with `--allow-scripts` (confirmed with 2.9.8).

**Change:** use `*.{js,cjs,mjs}`, as the Node and Bun toolchains do.

**Tests:** assert the globs in `tier1_test.rs`.

### #9 Small fixes

- [ ] `parse_deno_lock` (`toolchains/javascript/src/lockfiles/deno.rs:12-47`): a v3 lockfile (Deno
      1.x) nests its sections under `packages`, so `LockfileContent::from_json` returns no
      packages, and nothing is logged (confirmed with deno_lockfile 0.51.0). ADR-0008.9 requires a
      `debug!` when the `version` isn't 4 or 5. Deno 2 rewrites v3 files on its next install.
- [ ] `DENO_HOME` isn't a Deno variable: Deno's environment variable docs don't list it, and live,
      `DENO_HOME=<dir> deno install -g` wrote to `$HOME/.deno/bin`. Remove it from
      `tools/deno/src/proto.rs:330` (a `globals_lookup_dirs` entry that proto picks first when it
      has files) and from `toolchains/deno/src/tier2.rs:33` (with #3).
- [ ] `detect_package_manager` (`toolchains/javascript/src/tier1.rs:155-179`) only detects Deno from
      `deno.lock` (`:166`), so a new Deno repo gets the prompt. Detect `deno.json` and `deno.jsonc`
      last, after every lockfile, so a Node repo with a stray `deno.json` keeps its lockfile's
      manager.
- [x] `tools/deno/src/proto.rs:58`: `parse_version_file` parses the whole `package.json` as
      `PackageJson`, so an invalid `version` or dependency specifier silently drops the
      `devEngines.runtime` pin. Switch to the version-fields struct that `NODE.md` #1 adds to
      `lang-javascript-common` (handed over by `NODE.md`, which reproduced it for `node_tool`).
      Done in the JavaScript pass: `nodejs_package_json` now skips fields it can't parse.
- [ ] `toolchains/deno/tests/tier2_test.rs:118`, `:145` and `:200` call `create_toolchain("rust")`.
      The sandbox still loads the Deno WASM, but the ID is misleading. Use `deno`.

## Phase 2: after decisions

### #10 `deno.bins` are never installed (D1)

- [ ] Run the bin installs from the JavaScript toolchain's `setup_environment`
- [ ] Remove the Deno toolchain's `setup_environment`
- [ ] Tests
- [ ] Changelog

**Problem:** moon only schedules a toolchain's `SetupEnvironment` from
`internal_install_dependencies`, after checking `supports_tier_2()`, which means "has
`locate_dependencies_root`" (moon
`crates/toolchain-plugin/src/toolchain_plugin.rs:205-207`,
`crates/action-graph/src/action_graph_builder.rs:412-414`). `moon setup` needs a located root too
(`action_graph_builder.rs:1474-1481`). The Deno toolchain has no `locate_dependencies_root`, since
the JavaScript toolchain owns Deno's root, so its `setup_environment`
(`toolchains/deno/src/tier2.rs:141-191`) never runs. This is plugins#96, for Node's
`syncVersionManagerConfig`. **Live:** with `bins: ['jsr:@std/http/file-server']`, `moon run app:hi`
and `moon setup` added `SetupToolchain(deno)`, `SetupToolchain(javascript)` and
`SetupEnvironment(javascript)`, but no `SetupEnvironment(deno)`. `DENO_INSTALL_ROOT` stayed empty.

**Recommended:** in the JavaScript toolchain's `setup_environment`, when `packageManager` is
`deno`, load the `deno` config (`load_project_toolchain_config` or `load_toolchain_config`, as
`install_dependencies` does at `tier2.rs:420-423`) and emit the bin commands, moved from the Deno
toolchain. `SetupEnvironment(javascript)` already runs at every JavaScript dependencies root (live),
and moon runs identical memory-cached commands once per process, so several roots don't install
twice. The `bins` setting stays in the `deno` config.

**Alternative A:** give the Deno toolchain a `locate_dependencies_root`, duplicating the Deno arm of
the JavaScript toolchain's, with `workspace` members. moon then creates `SetupEnvironment(deno)`
(`action_graph_builder.rs:442-449`) and calls the Deno toolchain's `parse_manifest` (#12). Its
`parse_lock` would need the `deno.lock` parser, moved to `crates/lang-javascript-common`.

**Alternative B:** the moon change in `MOON_COMPACT.md`, which helps any toolchain in this shape.

**Tests:** move `setup_env_bins` from `toolchains/deno/tests/tier2_test.rs` to the JavaScript
toolchain, with a mock that asserts the `deno` ID. Update `does_nothing_if_deno`
(`tier2_env_test.rs:163`).

**Changelog:** `deno.bins` were never installed by moon v2. They now are, on the first run.

### #11 Make bin installs idempotent (with #10)

- [ ] Always pass `--force`
- [ ] Tests

**Problem:** `deno install --global` refuses to overwrite: running the same install twice fails with
"Existing installation found. Aborting (Use -f to overwrite)." (confirmed with 2.9.8). `--force` is
only passed when a bin sets `force` (`toolchains/deno/src/tier2.rs:171-173`). moon fingerprints the
returned commands (moon `crates/actions/src/actions/setup_environment.rs:94-114`), so the commands
re-run whenever the list changes. Adding a second bin would then fail setup on the first.

**Change:** always pass `--force`. A bin deleted by hand isn't reinstalled until the list changes,
which the Go audit accepted (Go D5).

**Tests:** expect `--force` in every bin command.

### #12 `deno.json` imports never reach task hashes (D1)

- [ ] Parse `deno.json` and `deno.jsonc` in the JavaScript toolchain
- [ ] Key imports like the lockfile (#5)
- [ ] Remove the Deno toolchain's `parse_manifest`
- [ ] Tests
- [ ] Changelog

**Problem:** moon hashes a task's dependency versions per toolchain, starting from that toolchain's
`locate_dependencies_root`, and skips the toolchain when there's none (moon
`crates/task-hasher/src/task_hashing.rs:255-266`):

- The Deno toolchain's `parse_manifest` (`toolchains/deno/src/tier2.rs:84-139`) is never called,
  for lack of a root (#10). The Unreleased fix for its versions has no effect yet.
- The JavaScript toolchain locates the root and parses `deno.lock`, but its only manifest is
  `package.json` (`toolchains/javascript/src/tier1.rs:55`, `tier2.rs:790`).

**Live** (the workspace repo): the `app:hi` hash manifest had no dependency entries for either
toolchain, and the root `deno.lock` isn't a task input, so `deno update` within the declared
ranges never changes the hash. The task inputs only had `apps/app/deno.json`, `main.ts` and
`.moon/toolchains.yml`.

The Deno parser would also miss once called. It keys by the import map's alias, not the package
(`:122-124`). A subpath like `jsr:@std/path@^1.0.0/join` gives the version `^1.0.0/join`, which is
skipped (`:105-117`). `catalog:` gets no version (`:118-120`), though Deno 2.9 resolves it from the
root's `catalog`.

**Recommended (D1):**

- Add `deno.json` and `deno.jsonc` to the JavaScript toolchain's `manifest_file_names`. They're
  already in its `config_file_globs`.
- In `parse_manifest`, dispatch on the file name. For `deno.json`, report each `imports` entry as
  `jsr:<name>` or the bare npm name (#5), with the version cut at the first `/` after it. Resolve
  `catalog:` through `load_catalogs`, as `package.json` does.
- Members use the root's `imports` too (confirmed: a member imported a root alias), so when
  `input.root` has a `deno.json` that isn't this file, merge its imports under the member's.
- Remove the Deno toolchain's `parse_manifest` and `src/deno_json.rs`. moon v2.6.5 doesn't read
  `publishable`.

Paths and URLs stay as they are. URL imports are pinned in the URL, and `deno.json` is already a
task input.

With `deno.json` in its manifest names, moon also parses changed `deno.json` files into the install
fingerprint (moon `install_dependencies.rs:300-319`).

**Tests:** a `deno.json` with `jsr:`, `npm:`, a subpath, an alias, `catalog:` and a URL, and a
member that inherits the root's imports. Live: a lockfile bump changes the task hash.

**Changelog:** Deno task hashes change once.

### #13 Projects with only a `deno.json` aren't in the project graph (D2)

- [ ] Collect `deno.json` projects
- [ ] Alias and edges from `deno.json`
- [ ] Infer `deno.json` tasks and `package.json` scripts
- [ ] Tests

**Problem:** `extend_project_graph`'s first pass only collects projects with a `package.json`
(`toolchains/javascript/src/tier2.rs:30-58`), and the second pass, which infers Deno tasks
(`:112-127`), only walks those. A Deno workspace usually has no `package.json`. **Live** (the
workspace repo, `inferTasksFromScripts: true`):

- `moon project app` showed "Depends on: —", though `main.ts` imports `@x/lib`, and no `start`
  task from `deno.json`.
- `moon docker scaffold app` left `packages/lib` out of the sources skeleton, so the image can't run
  the app.
- moon's Deno handbook says edges come from `imports`.

Under the Deno package manager, `deno task` also runs `package.json` scripts (confirmed), but only
`deno.json` tasks are inferred.

**Recommended:**

- When `packageManager` is `deno`, also collect projects with a `deno.json` or `deno.jsonc`, with
  `name` from it (falling back to `package.json`'s).
- Link an `imports` entry whose value is `jsr:<name>` or `npm:<name>`, with any version, when
  `<name>` is another project's name, as `Production`. Deno resolves these to the local member
  (confirmed with `jsr:@scope/b@^1.0.0`).
- Infer `deno.json` tasks (#4) for every collected project, plus `package.json` scripts as
  `deno task <name>` when `deno.json` has no task with that name.

**Not covered:** members imported by name in source files without an `imports` entry, as `app` does.
Deno resolves those, but finding them needs `deno info --json` per project at graph time, which is
Go #19's cost. Document declaring workspace dependencies in `imports`.

**Tests:** a fixture with `deno.json`-only members, a `jsr:` import of a member, and inferred tasks.

## Phase 3: lower priority

### #14 Every run reinstalls a `deno.json`-only workspace

- [ ] Wait for moon (see `MOON_COMPACT.md`)

The JavaScript toolchain's `vendor_dir_name` is `node_modules`. Without a `package.json`, Deno's
`nodeModulesDir` defaults to `none`, so `deno install` never creates it (confirmed with 2.9.8). moon
then treats every run as a first install (moon `install_dependencies.rs:100-101`): it forces the
install past the fingerprint (`:174`), and installs in `--affected` runs and VCS hooks too
(`:106`). **Live:** three runs in a row each logged "Installing JavaScript dependencies". A cached
`deno install` took 70–110 ms.

There's no plugin fix: `register_toolchain` gets no config, and `vendor_dir_name` is shared by every
package manager. It needs moon's "Let plugins report whether dependencies are installed".

### #15 Detection names

- [ ] `exe_names` (`toolchains/deno/src/tier1.rs:17`, and the JavaScript toolchain's
      `tier1.rs:73-96`): add `dx`, which `deno x --install-alias` creates, as Bun lists `bunx`.

## Not planned

- **Hashing `.dvmrc` when `version` is unset:** moon hashes `version` when it's set, and Ruby #8
  removed the equivalent.
- **The default bin permissions** (`--allow-net --allow-read`, `tier2.rs:154`): `args` (1.1.3) can
  add more, and changing the defaults would change installed bins.
- **Edges from `deno info`:** see #13.
- **Parsing v3 lockfiles:** only Deno 1.x writes them (#9 logs them).
- **Setting `DENO_DIR`:** Deno's cache lives outside the workspace, and the official image sets it.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added "Schedule
`SetupEnvironment` for toolchains without a dependencies root" (#10's Alternative B), the Deno case
in "Let plugins report whether dependencies are installed" (#14), "`moon docker file` without a TTY
writes `FROM scratch`", and the Deno website docs.

**proto:**

- [ ] With no populated globals directory, `locate_globals_dir` falls back to the last lookup
      entry (`proto_core` 0.64.2 `flow/locate.rs:425-434`). For Deno that's `$HOME/.deno/bin` even
      when `DENO_INSTALL_ROOT` is set, where `deno install -g` writes. #3 works around it in the
      toolchain.

**Deno:**

- [ ] `deno install --prod` and `deno ci --prod` still install `devDependencies` when a `deno.lock`
      exists (2.8.0 and 2.9.8): a `package.json` with `is-odd` and a dev `left-pad`, after a full
      install, `rm -rf node_modules`, then `deno install --prod`, installs both. Without a lockfile,
      `--prod` drops `left-pad`. So `docker prune` keeps dev dependencies in Deno images. Report it
      upstream with this repro.

## Release

**Blocked:** `deno_tool` on `master` sets `minimum_proto_version` to 0.64, and moon 2.6.4 and 2.6.5
embed proto_core 0.62, so they refuse to load a toolchain built from `master`
(`MOON_COMPACT.md`, "Ship proto 0.64"). The live checks above ran against the released plugins.

- `javascript_toolchain`: #1, #4, #5 and #6 change installs, inferred tasks and task hashes, so ship
  them as 1.4.0 with changelog callouts: frozen CI installs without a version, and a one-time hash
  change. #10, #12 and #13 can follow in 1.4.x or 1.5.0, with callouts for bins starting to
  install and another hash change.
- `deno_toolchain`: #2, #3, #7 and #8 change task command lines, `PATH` and Docker output, so ship
  them as 1.2.0. Removing `setup_environment` and `parse_manifest` for D1 belongs in the same
  release as the JavaScript toolchain's #10 and #12.
- `deno_tool`: #9's `DENO_HOME` removal and `package.json` parse are a patch (0.16.1). The toolchain re-exports the tool, so
  it also needs a toolchain release.

## Handed to other plans

- `JAVASCRIPT.md`: `version_satisfies` returns `false` for unset versions and aliases
  (`toolchains/javascript/src/config.rs:153-175`), which turns version-gated behavior off for
  `version: latest` or no version, like `bun ci` and Bun 1.4's `--filter` patterns.
- `JAVASCRIPT.md`: `is_valid` drops every `package.json` script starting with `pre` or `post`
  (`infer_tasks.rs:196-206`), so `preview`, `prettier` and `postcss` are never inferred, and
  `is_dev_script_name`'s `preview` check can't match.
- `JAVASCRIPT.md`: `lock_file_names` lists `bun.lock` and `bun.lockb` before `deno.lock`
  (`tier1.rs:56-71`), so a repo with both fingerprints installs with the Bun lockfile.
- `JAVASCRIPT.md`: the install tests mock `load_toolchain_config` with `|_, _|`, ignoring the
  requested ID, for every package manager.
