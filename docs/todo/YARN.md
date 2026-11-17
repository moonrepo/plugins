# Yarn toolchain fix plan

Fixes from the Yarn toolchain audit (2026-11-17, against moon v2.6.5 source, live-checked with moon
2.6.4, proto 0.64.0, Node 24.22.3, and Yarn 1.22.22, 3.6.3, 4.18.1 and 6.0.0-rc.23, the latest of
each line; Yarn 5 was never released). Covers the Yarn arms of:

- `toolchains/node-depman` (`node_depman_toolchain` 1.1.1, toolchain ID `yarn`): registration,
  `YarnToolchainConfig`, `define_requirements` and `setup_environment` for `PackageManager::Yarn1`,
  `Yarn2to5` and `Yarn6`.
- `toolchains/javascript` (`javascript_toolchain` 1.3.2, `javascript` with `packageManager: yarn`):
  installs, dedupe, workspaces, catalogs, `yarn.lock`, project sync, the project graph and Docker.

`tools/node-depman` (`node_depman_tool` 0.21.0 on `master`, 0.20.2 in the release) is re-exported as
tier 3, was audited on 2026-11-06, and is only covered here where the toolchains depend on it. Item
numbers (#1–#18) match the audit findings.

What changed upstream: Yarn 1.22.22 (March 2024) is the last 1.x. Yarn 4.10 added catalogs in
`.yarnrc.yml`, and 4.14 made `enableScripts: false` the default and added `approvedGitRepositories`,
which blocks git dependencies by default (seen with 4.18.1). Current 4.x lockfiles are
`__metadata.version: 10`, while this repo's fixture is `version: 8`. Yarn 6 (zpm, written in Rust)
is still a release candidate (rc.23, 2026-10-14). It writes `yarn.lock` as JSON, defaults to PnP,
keeps its install state in `.yarn/ignore/`, has no plugin system and no `install --production`, and
doesn't freeze the lockfile in CI. Its archive ships `yarn`, which is Yarn Switch (a version
manager), next to `yarn-bin`.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build. Build the tool in its own run.

```bash
cargo build -p javascript_toolchain -p node_depman_toolchain --target wasm32-wasip1 --release
cargo build -p node_depman_tool --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p javascript_toolchain -p node_depman_toolchain -p node_depman_tool --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

Findings marked **Live** were reproduced with `~/.proto/tools/moon/2.6.4/moon` in scratch git repos:
a root `package.json` with `workspaces: ["packages/*"]`, and projects `packages/a` (depends on `b`,
plus `is-number` and a dev `is-odd`) and `packages/b`. moon 2.6.4 loads the released plugins. Their
Yarn code matches `master`: since `javascript_toolchain-v1.3.2`, only lockfile error handling
changed (ADR-0008), and the node-depman toolchain's Yarn code hasn't changed. A `master` build of
`javascript_toolchain` also loads in moon 2.6.4, as it embeds no tool, and #2 was confirmed on it.

Gotchas for live checks:

- moon puts `~/.proto/shims` on PATH, and a global `~/.proto/.prototools` that pins `yarn` makes
  that shim run its version. To check an unset `yarn.version`, use a scratch `PROTO_HOME` and
  `yarn: { versionFromPrototools: false }`, and put the Yarn under test first on PATH.
- Yarn reads `~/.yarnrc.yml`, so isolate `HOME` when running Yarn directly.
- `moon docker setup` can be emulated without Docker: copy `.moon/docker/configs` to a fresh dir,
  commit it, run `moon docker setup`, copy `.moon/docker/sources` over it, and run
  `moon docker prune`.

Lockfile parsing was checked with a scratch crate that runs yarn-lock-parser 0.14.0 and the
plugin's filtering, over lockfiles written by each Yarn version, plus Berry's own `yarn.lock` (22k
lines) and moon's (17k lines).

**Checked, no change needed:**

- yarn-lock-parser 0.14.0 parses Yarn 1 lockfiles (aliases, git, `file:`, `link:`, the object
  `workspaces` form with `nohoist`) and Berry's YAML at metadata versions 6 (Yarn 3) and 10
  (Yarn 4), including `npm:` aliases, `patch:` (Yarn's builtin TypeScript patch), `portal:`,
  `link:`, `file:`, `exec:`, git, `conditions:` and multi-descriptor keys. Berry's and moon's own
  lockfiles parse with no errors. Workspace and unnamed-root entries are skipped, and lockfiles of
  projects without dependencies parse.
- `.yarnrc.yml` parsing (`YarnRc`) accepts an empty file, comments only, unquoted numbers
  (`is-odd: 3`, which Yarn 4 accepts) and `npm:` aliases.
- Workspace members come from both `workspaces` forms, and `./packages/*` globs still match.
- Yarn 4.10+ and Yarn 6 read `catalog` and `catalogs` from `.yarnrc.yml`, which is where the plugin
  looks (`toolchains/javascript/src/tier2.rs:283-291`). See #7 for its test.
- `yarn workspaces focus <packages> --production` drops dev dependencies on Yarn 4 (live through
  `docker prune`) and Yarn 6, and `--all --production` works on both.
- Yarn 2–4 freeze the lockfile in CI on their own (`enableImmutableInstalls`): 4.18.1 failed with
  YN0028 on a stale lockfile with `CI=true`.
- `yarn dedupe` works on Yarn 3, 4 and 6. With `version: '1.22.22'`, moon ran
  `npx --quiet --package yarn-deduplicate -- yarn-deduplicate yarn.lock`, which passed.
- Inferred tasks run `yarn run <script>` (`toolchains/javascript/src/infer_tasks.rs:96-105`), which
  works under PnP.
- The `.yarn/**/*` scaffold glob copies `.yarn/releases` (for `yarnPath`) into the configs
  skeleton (live), and with it `.yarn/patches`, `.yarn/plugins` and a zero-install `.yarn/cache`.
- The node toolchain's `*.{js,cjs,mjs}` scaffold glob copies `.pnp.cjs` and `.pnp.loader.mjs`
  (live). That's harmless: Yarn 4 writes the same `.pnp.cjs` for darwin and linux targets, as the
  platform only decides what's fetched.
- Requiring `node` for Yarn 6 (`toolchains/node-depman/src/tier2.rs:10-24`) is still right, as
  lifecycle scripts and the PnP runtime need it.

## Decisions needed

| Decision                                                             | Recommendation                                                                  |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| D1 (#9): freeze `yarn.lock` in CI for Yarn 1 and Yarn 6?             | Yes: `--frozen-lockfile` and `--immutable` when it exists. Yarn 2–4 already do  |
| D2 (#10): fix or deprecate `yarn.plugins`?                           | Deprecate. It has never run in moon v2, and Yarn 4+ bundles the official ones  |
| D3 (#11): should `yarn.version` win over a committed `yarnPath`?     | Yes: `YARN_IGNORE_PATH=1` from the tool's `activate_environment`                |
| D4 (#12): link workspace packages declared by a version range?       | Yes; it's `JAVASCRIPT.md` D1, which already includes Yarn                       |
| D5 (#13): give PnP tasks Yarn's runtime?                             | No: document `yarn node`, `yarn <bin>` and `nodeLinker: node-modules`           |

## Phase 1: clear-cut fixes

### #1 Yarn 6 runs Yarn Switch, which picks another Yarn

- [ ] Keep Yarn Switch off PATH
- [ ] Tests
- [ ] Changelog (tool and toolchain)

**Problem:** Yarn 6's archive has two binaries: `yarn-bin`, the package manager, and `yarn`, which
is Yarn Switch. The switch runs whatever `packageManager` in the closest `package.json` asks for,
and Yarn 1 when nothing does (zpm `packages/zpm-switch/src/commands/proxy.rs:52-55` at
v6.0.0-rc.23). The tool registers `yarn-bin` as the primary executable
(`tools/node-depman/src/proto.rs:538-540`), so proto's shims are right. But moon adds the
executable's parent directory to PATH (moon
`crates/toolchain-plugin/src/toolchain_plugin.rs:235-237`), which is the install directory, so
moon's `yarn install` and every task's `yarn` run the switch.

**Live:**

- With `yarn.version: '^6.0.0-rc.0'` and no `packageManager`, `moon run` printed
  `yarn install v1.22.22` and failed on `workspace:^`.
- A task's `which -a yarn` listed `$PROTO_HOME/tools/yarn/6.0.0-rc.23/yarn` first.
- With `packageManager: yarn@4.18.1`, the switch in the 6.0.0-rc.23 install printed `4.18.1`.

`syncPackageManagerField` hides this when `version` is exact, since it writes `packageManager`
before the install (`toolchains/javascript/src/tier2.rs:855-872`). It skips aliases and requirements
like `rust`, `canary` and `^6.0.0-rc.0`, though, and can be turned off.

**Change:** in `locate_executables`, for `Yarn6`, remove `<install_dir>/yarn` (`yarn.exe` on
Windows) when `yarn-bin` exists. The function already writes shims there lazily for Yarn 1–5
(`proto.rs:479-481`), so this also fixes existing installs. The switch's own commands
(`yarn switch …`) go away, which is fine while proto manages the version.

**Alternative:** point the primary executable at an internal `shims/yarn` that runs `../yarn-bin`,
so the install directory never reaches PATH. That needs a shim template that doesn't run `node`.

**Tests** (`tools/node-depman/tests`): a Yarn 6 install directory with both binaries keeps only
`yarn-bin` after `locate_executables`.

### #2 Yarn 6 lockfiles fail every task hash

- [ ] Parse the JSON `yarn.lock`
- [ ] Return nothing for an empty `yarn.lock`
- [ ] Unit tests (ADR-0006)
- [ ] Changelog

**Problem:** Yarn 6 writes `yarn.lock` as JSON, like `{"__metadata":{"version":10},"entries":{…}}`
(zpm `packages/zpm/src/lockfile.rs:543-565`). `parse_yarn_lock_content`
(`toolchains/javascript/src/lockfiles/yarn.rs:10-49`) only handles Yarn 1's format and Berry's YAML,
through yarn-lock-parser 0.14.0, which rejects it. moon parses lockfiles for every task hash when
`hasher.optimization` is `accuracy` (moon `crates/task-hasher/src/task_hashing.rs:269-285`), and
that's the default (`crates/config/src/workspace/hasher_config.rs:8-11`).

**Live:** with Yarn 6.0.0-rc.23 and the default config, every task failed with "Error parsing
yarn.lock file" on the released 1.3.2, and with "Failed to parse /workspace/yarn.lock: Error parsing
yarn.lock file" on a `master` build.

An empty `yarn.lock` fails the same way (confirmed). Berry's docs suggest one to make a nested
directory its own project, and it stays empty until the first install writes it.

**Change:**

- When the trimmed contents start with `{`, deserialize into our own types, holding only `entries`
  and `islands` (a map of island IDs to entries). Each value is
  `{ checksum: Option<String>, resolution: { resolution: String, version: String } }` (ADR-0008.2).
  Deserialize each entry on its own, and skip one that fails with `debug!` (ADR-0008.5).
- Take the package name from the `resolution` locator (`is-number@npm:7.0.0`) with
  `parse_name_and_version` (`lockfiles/mod.rs:34-67`), not from the key, which can start with an
  alias (#6). Skip `@workspace:` locators, use the checksum as the `hash`, and parse the version
  with `parse_version_spec`.
- Return no dependencies for an empty file.

**Tests** (`#[cfg(test)]` in `lockfiles/yarn.rs`, which the `test` task builds, ADR-0006): a trimmed
rc.23 lockfile with a workspace entry, the root, a multi-descriptor key and a `null` checksum; an
entry with an unknown shape (skipped); and an empty file.

### #3 Tell Yarn versions apart, so Yarn 1 doesn't run `yarn dedupe`

- [ ] Add a helper that returns Yarn's major version
- [ ] Use it for dedupe and focused installs
- [ ] Unit tests
- [ ] Fix the Yarn 1 dedupe test

**Problem:** the Yarn arms pick commands with `SharedPackageManagerConfig::version_satisfies`
(`toolchains/javascript/src/tier2.rs:570`, `:648`). It returns `false` when `version` is unset or an
alias (`toolchains/javascript/src/config.rs:153-174`), and for every prerelease: version_spec
follows semver, so `>=2.0.0` doesn't match `6.0.0-rc.23` (version_spec 0.11.8
`syntax_traits.rs:19-23`), and every Yarn 6 release is a prerelease. moon's own setup docs configure
`yarn: {}` (moon `website/docs/__partials__/setup-toolchain/node/tier2.mdx`).

With Yarn 1 and no `version`, dedupe is `yarn dedupe`, which Yarn 1 rejects with exit 1: "The dedupe
command isn't necessary. `yarn install` will already dedupe." **Live** (Yarn 1.22.22 on PATH,
`yarn: { versionFromPrototools: false }`): the first install passed, since moon skips dedupe on a
first install. Every later run failed in `InstallDependencies` at `yarn dedupe`, including runs with
no changes, as a failed action stores no fingerprint. `version: 'classic'` takes the same path.

**Change:** add a pure helper (in `config.rs` or a new ungated module, ADR-0006) that returns Yarn's
major version:

- For an exact version or a requirement, its major, ignoring prereleases.
- `canary`, `rust` and `zpm` are 6, and `classic` and `legacy` are 1. Any other alias (`latest`,
  `berry`, `stable`) is 4, which is what `node_depman_tool` resolves them to
  (`tools/node-depman/src/proto.rs:332-347`).
- When `version` is unset or a range, read `<root>/yarn.lock`: JSON is 6, a `__metadata:` line is 4,
  and anything else is 1, which is also the answer without a lockfile.

Use it for the dedupe choice (`yarn-deduplicate` for 1), and in #4, #5, #9 and #15. Reading
`packageManager` from the root `package.json` would also cover a Berry repo before its first
lockfile, but nothing needs that today.

**Tests:** the helper for each case above. In `toolchains/javascript/tests/tier2_test.rs`,
`legacy_dedupe_for_older_versions` in `mod yarn` (`:2157-2194`) configures `packageManager: "pnpm"`,
so the Yarn 1 command has no test. Point it at Yarn, and add an unset version with a v1 lockfile
(`yarn-deduplicate`) and with a Berry lockfile (`yarn dedupe`).

### #4 Production installs on Yarn 2+ use a removed flag

- [ ] Use `yarn workspaces focus` for every production install on Yarn 2+
- [ ] Tests

**Problem:** `docker prune` installs with `production: true`, and passes the focused projects'
package names as `packages` (moon `crates/app/src/commands/docker/prune.rs:174-190`, `:225-238`).
The plugin only uses `yarn workspaces focus` when `packages` isn't empty and
`version_satisfies(">=2.0.0")` (`tier2.rs:570-575`), and otherwise runs `yarn install --production`
(`:577-579`). Yarn 2+ removed that flag:

- Yarn 3.6.3 and 4.18.1 exit 1 with "YN0050: The --production option is deprecated on 'install'; use
  'yarn workspaces focus' instead".
- Yarn 6 exits 1 with "The provided arguments don't match any known syntax".

So it fails with an unset `version`, an alias, any Yarn 6 version (#3), or a focused project without
a package name. **Live:** with Yarn 4.18.1 and no `version`, and with `version: '6.0.0-rc.23'`,
`moon docker prune` failed at `yarn install --production`. With `version: '4.18.1'` it ran
`yarn workspaces focus a --production`, and `node_modules` kept only `a`, `b` and `is-number`.

**Change:** when #3's major is 2 or more and `production` is set, run
`yarn workspaces focus <packages> --production`, with `--all` instead of the packages when there are
none. `--all --production` is Yarn's documented replacement for `yarn install --production`, and
worked on 4.18.1 and 6.0.0-rc.23. Keep the non-production focus, and `yarn install --production`
for Yarn 1. Yarn 2 and 3 need the workspace-tools plugin for `focus` (#15).

**Tests:** `focused_commands` (`tier2_test.rs:2065-2089`) asserts `yarn install --production` for an
unset version; give it a v1 lockfile. Add Berry with no packages (`--all`), `6.0.0-rc.23`, and an
alias.

### #5 Yarn 1 can't install synced `workspace:*` dependencies

- [ ] Map the workspace formats for Yarn 1
- [ ] Tests

**Problem:** `syncProjectWorkspaceDependencies`, which is on by default, adds a `package.json`
dependency for each moon project dependency, in `dependencyVersionFormat`, which defaults to
`workspace` (`workspace:*`). `get_supported_for` maps the workspace formats for npm (to `file:`) and
Bun, but keeps them for every Yarn (`toolchains/javascript/src/config.rs:75-96`), and Yarn 1 has no
`workspace:` protocol.

**Live** (Yarn 1.22.22, `a` with `dependsOn: [b]`): sync wrote `"b": "workspace:*"` into
`packages/a/package.json`, and the install failed with `Couldn't find any versions for "b" that
matches "workspace:*"`. Sync only adds missing dependencies, so the entry stays until it's removed
by hand.

**Change:** in `sync_project_workspace_dependencies` (`toolchains/javascript/src/tier1.rs:240-321`),
when the package manager is Yarn and #3's major is 1, map `workspace` to `star`, `workspace-caret`
to `version-caret` and `workspace-tilde` to `version-tilde`. Yarn 1 links a workspace for both `*`
and `^1.0.0` (confirmed: `node_modules/b` was a symlink to `packages/b`, with no lockfile entry).
`get_supported_for` only takes the package manager, so do it next to the call (`:247-249`).

**Tests** (`tier1_sync_test.rs`): Yarn 1 with each workspace format, and Yarn 4 keeping
`workspace:*`.

### #6 Aliased lockfile entries hide the real package

- [ ] Record each entry under every descriptor's name
- [ ] Tests

**Problem:** Berry merges an alias and the package it points at into one entry when both resolve to
the same locator. Keys are sorted, so the alias often comes first:
`"alias-num@npm:is-number@7.0.0, is-number@npm:7.0.0, is-number@npm:^7.0.0"`. yarn-lock-parser names
an entry after its first descriptor, and the plugin records it under that name only
(`toolchains/javascript/src/lockfiles/yarn.rs:26-45`). So `is-number`'s 7.0.0 entry is missing, and
moon's hasher finds no locked version for `"is-number": "^7.0.0"` and records the requirement
instead (moon `task_hashing.rs:433-480`). Upgrades within the range then don't change task hashes.

Confirmed with Yarn 4.18.1, and in moon's own `yarn.lock`, where `string-width` and `strip-ansi` are
only recorded as `string-width-cjs` and `strip-ansi-cjs`. Those aliases come from `@isaacs/cliui`,
which most `glob` installs pull in.

**Change:** push the dependency under each distinct name in `entry.descriptors`, instead of
`entry.name`. The alias names are harmless extras, since `parse_manifest` skips `npm:` aliases
(`toolchains/javascript/src/tier2.rs:719`). Yarn 1.22.22 wrote the alias as its own entry
(confirmed), so this only changes Berry lockfiles.

**Tests:** a lockfile excerpt with a merged alias key yields entries for both names.

### #7 Tests that miss the Yarn paths

- [ ] Rename the catalogs fixture's `.yarnc.yml`
- [ ] Assert the requested toolchain ID in mocks
- [ ] Rename `inherits_args_from_bun_toolchain`

- The Yarn file in `toolchains/javascript/tests/__fixtures__/catalogs` is named `.yarnc.yml`, so
  `supports_catalogs_yarn` (`tier2_test.rs:2415`) passes with the catalogs from `package.json`, and
  `YarnRc` (`lockfiles/yarn.rs:57-79`) is never read. Rename it to `.yarnrc.yml`, and give it
  versions that differ from `package.json`'s, so the test shows that `.yarnrc.yml` is used. Yarn
  4.10+ and Yarn 6 read catalogs from `.yarnrc.yml` (confirmed with 4.18.1 and 6.0.0-rc.23).
- The Yarn tests mock `load_toolchain_config` with `|_, _|` (`:2097`, `:2132`, `:2162`, `:2201`),
  which ignores the requested ID, as `PYTHON.md` found. Assert `yarn`.
- `inherits_args_from_bun_toolchain` (`:2127`) tests Yarn.

#3 and #4 cover the dedupe and production tests.

### #8 Small fixes

- [ ] `parse_yarn_lock_content` skips entries whose name contains `root-workspace` (`yarn.rs:19`).
      The unnamed root's resolution, `root-workspace-0b6124@workspace:.`, is already skipped by the
      `workspace:` check, so drop the name check, which also skips any package with
      `root-workspace` in its name.
- [ ] `YarnToolchainConfig.install_args` (`toolchains/node-depman/src/config.rs:52-53`) says it's
      appended to `yarn install`. Say that `yarn workspaces focus` doesn't get it
      (`toolchains/javascript/src/tier2.rs:574`).

## Phase 2: after decisions

### #9 Frozen installs in CI for Yarn 1 and Yarn 6 (D1)

- [ ] Pass `--frozen-lockfile` (Yarn 1) or `--immutable` (Yarn 6) in CI
- [ ] Tests
- [ ] Changelog

**Problem:** the Yarn arm runs a plain `yarn install` in CI (`tier2.rs:567-582`), while npm, Bun,
Deno and Nub switch to frozen installs (`:432`, `:469`, `:493`, `:518`). Yarn 2–4 freeze the
lockfile in CI themselves through `enableImmutableInstalls`. Yarn 1 doesn't, and neither does Yarn
6.0.0-rc.23: `yarn config get enableImmutableInstalls` printed `false` with `CI=true` and
`GITHUB_ACTIONS=true`.

**Live** (Yarn 1.22.22): after a member's `package.json` gained a dependency without a lockfile
update, `CI=true moon run` on a fresh checkout passed and left `yarn.lock` modified. Yarn 6 rewrote
a stale `yarn.lock` with `CI=true` and exited 0, and failed with `--immutable`.

**Recommended:** when `env.ci` is set and `<root>/yarn.lock` exists, add `--frozen-lockfile` for
Yarn 1 and `--immutable` for Yarn 6 (#3), unless `installArgs` already has `--frozen-lockfile`,
`--pure-lockfile`, `--immutable` or `--no-immutable`. Leave Yarn 2–4 to Yarn, so
`enableImmutableInstalls: false` in `.yarnrc.yml` keeps working.

**Not covered:** Yarn 1's `--frozen-lockfile` misses changes in workspace members
(yarnpkg/yarn#5840). Confirmed: a new dependency in a member exited 0, while the same change in the
root or in a single package failed with "Your lockfile needs to be updated".

**Tests:** CI with and without `yarn.lock`, for Yarn 1, 4 (no flag) and 6, and `installArgs` with
the flag.

**Changelog:** CI installs with a stale `yarn.lock` now fail on Yarn 1 and 6.

### #10 `yarn.plugins` never runs (D2)

- [ ] Deprecate `plugins`
- [ ] Remove the unreachable `setup_environment`

**Problem:** the setting is applied by the Yarn toolchain's `setup_environment`
(`toolchains/node-depman/src/tier2.rs:26-60`). moon only schedules `SetupEnvironment` for a
toolchain from its located dependencies root (moon
`crates/action-graph/src/action_graph_builder.rs:421-449`, and `:1465-1481` for `moon setup`).
node-depman has no `locate_dependencies_root`, so it isn't tier 2 (`toolchain_plugin.rs:205-207`).
This is how `syncVersionManagerConfig` broke for Node (plugins#96).

**Live** (Yarn 3.6.3, `plugins: ['workspace-tools']`): neither `moon run` nor `moon setup` added
`SetupEnvironment(yarn)`, and `.yarn/plugins` was never created.

If it were scheduled, it would still be broken:

- It returns the same commands on every run, so moon only runs them once (plugins#137).
- It only runs for Yarn 2–5 with `version` set, and Yarn 4 bundles every official plugin:
  `yarn plugin import workspace-tools` exits 1 with "possibly attempting to import a built-in
  plugin" (confirmed with 4.18.1).
- Yarn 6 has no `yarn plugin` command (confirmed).

**Recommended:** mark `plugins` as `#[deprecated]`, described as "Unused. Yarn 4 and later bundle
the official plugins. For Yarn 2 and 3, commit `.yarn/plugins` and `.yarnrc.yml`.", like Go's
`vendorDir`. Remove `setup_environment` and its tests
(`toolchains/node-depman/tests/tier2_test.rs`), which pass but test a function moon never calls. The
`.yarn/**/*` scaffold glob already copies committed plugins into Docker.

**Alternative:** move it to the JavaScript toolchain's `setup_environment`, as plugins#96 did, for
Yarn 2 and 3 only. Return `yarn plugin import <name>` only when
`.yarn/plugins/@yarnpkg/plugin-<name>.cjs` is missing, so the commands change with the disk.

### #11 `yarnPath` overrides `yarn.version` (D3)

- [ ] Set `YARN_IGNORE_PATH=1` when `version` is set
- [ ] Tests
- [ ] Changelog

**Problem:** Berry repos often commit a release in `.yarn/releases` and point `yarnPath` at it,
which is what `yarn set version` writes. Every Yarn binary hands off to `yarnPath`, so moon installs
and hashes `yarn.version` while another Yarn runs.

**Live:** with `version: '4.18.1'` and `yarnPath: .yarn/releases/yarn-3.6.3.cjs`, a `yarn --version`
task printed `3.6.3`. `moon docker prune` failed at `yarn workspaces focus` with "Command not
found", since the plugin decided from 4.18.1 that workspace-tools is built in. With
`YARN_IGNORE_PATH=1`, the same binary printed `4.18.1`.

**Recommended:** return `YARN_IGNORE_PATH=1` from the tool's `activate_environment`
(`tools/node-depman/src/proto.rs:590-657`) for Yarn. moon applies activation env to installs and
tasks only when `version` is set, and doesn't override a task's own env (moon
`crates/process-augment/src/augmented_command.rs:395-405`). proto applies it in `proto run` and
`proto activate` too, so a `.prototools` pin also wins over `yarnPath`. This matches Go's
`GOTOOLCHAIN` and Rust's `RUSTUP_TOOLCHAIN`.

**Alternative:** set it only in moon, from a new `extend_command` in the Yarn toolchain. That env
overrides a task's own `env`, though (`augmented_command.rs:20-27`, `:133-147`).

**Not covered:** a repo that keeps Yarn 1 as `version` and bootstraps Berry through `yarnPath` would
now run Yarn 1. Its `version` is wrong either way.

**Tests** (`tools/node-depman/tests/activate_test.rs`): Yarn gets the variable; npm doesn't.

**Changelog:** needs a callout, since repos whose `yarnPath` and `version` disagree change Yarn.

### #12 Workspace dependencies declared by version get no edge (D4)

- [ ] Land `JAVASCRIPT.md` #15 with Yarn included
- [ ] A Yarn 1 test with an exact version

**Problem:** `extend_project_graph` only links `*`, `workspace:`, `file:`, `link:` and `portal:`
(`toolchains/javascript/src/tier2.rs:77-89`). Yarn 1 has no `workspace:` protocol, so its workspaces
depend on each other by version, usually an exact one (`"b": "1.0.0"`), and Yarn links the local
package when its version satisfies it (confirmed in #5). Berry does the same while
`enableTransparentWorkspaces` is on, which is the default.

**Live** (Yarn 1.22.22, `"b": "1.0.0"`): `moon project a` showed no dependencies, while
`"b": "workspace:^"` with Yarn 4 showed `b`. `^:` task dependencies and affected detection miss
these edges.

**Recommended:** this is `JAVASCRIPT.md` #15 (its D1), which links `Version`, `Requirement` and
`Range` for npm, Yarn and Bun when the local package's `version` satisfies them, and found the same
with Yarn 4.18.1 and `^1.0.0`. Keep the `Version` case, which is how Yarn 1 repos write it. A Berry
repo with `enableTransparentWorkspaces: false` would get edges that Yarn doesn't link, but reading
`.yarnrc.yml` for it isn't worth it.

**Tests:** add Yarn 1 with an exact version to #15's tests.

### #13 PnP tasks can't resolve dependencies (D5)

- [ ] Document how to run tasks under PnP

**Problem:** `nodeLinker: pnp` is the default in Yarn 2+ and Yarn 6, and resolves packages through
`.pnp.cjs` instead of `node_modules`. Tasks get `node_modules/.bin` on PATH (`tier2.rs:139-176`),
which doesn't exist, and run `node` without PnP's loader.

**Live** (Yarn 4.18.1 with PnP): a `node -e 'require.resolve("is-number")'` task failed with "Cannot
find module 'is-number'". With `NODE_OPTIONS="--require <root>/.pnp.cjs"`, which `yarn node` sets,
it resolved. Inferred tasks run `yarn run <script>`, so they work.

**Recommended:** document it in moon's JavaScript docs: run tools through Yarn (`yarn node`,
`yarn eslint`), or use `nodeLinker: node-modules`, which moon's workspace docs already show (moon
`website/docs/__partials__/node/package-workspaces.mdx`).

**Alternative:** when `.pnp.cjs` exists at the dependencies root, add `--require <root>/.pnp.cjs`
(and `--experimental-loader <root>/.pnp.loader.mjs` when that exists) to the host's `NODE_OPTIONS`
in `extend_task_command` and `extend_task_script`. That covers `node` commands but not bare
binaries, and the plugin's env replaces a task's own `NODE_OPTIONS`.

## Phase 3: lower priority

### #14 PnP repos reinstall on every run

- [ ] moon: let plugins report whether dependencies are installed (see `MOON_COMPACT.md`)
- [ ] Report a Yarn PnP install as installed when `.pnp.cjs` exists

**Problem:** `vendor_dir_name` is `node_modules` (`toolchains/javascript/src/tier1.rs:97`), which
PnP never creates. moon then treats every run as a first install (moon
`crates/actions/src/actions/install_dependencies.rs:100-101`): it ignores the fingerprint, installs
under `--affected` and in VCS hooks (`:106-122`), and never dedupes (`:231`).

**Live** (Yarn 4.18.1): three `moon run` calls in a row each ran `yarn install` and logged
"Dependencies were installed for the first time, skipping dedupe". With `nodeLinker: node-modules`,
the same repo skipped the install on the third run. The install took about 0.3 s on the scratch
repo, and grows with the repo.

`register_toolchain` gets no config, so the plugin can't change `vendor_dir_name` by linker.

**Change:** once moon has the hook, report dependencies as installed for Yarn 2 and later (#3) when
`<root>/.pnp.cjs` exists.

### #15 Yarn 2 and 3: name the plugin that `focus` needs

- [ ] Return an error when workspace-tools is missing

**Problem:** Yarn 2 and 3 only have `yarn workspaces focus` with the workspace-tools plugin, so #4's
production install fails with "Unknown Syntax Error: Command not found" (confirmed with 3.6.3). moon
v1 checked for `.yarn/plugins/@yarnpkg/plugin-workspace-tools.cjs` first and named the fix (moon
`legacy/node/tool/src/yarn_tool.rs:293-303` at v1.41.8).

**Change:** for majors 2 and 3 (#3), when that file is missing, return an error that names
`yarn plugin import workspace-tools`. After #10's deprecation, it's the only pointer users get.

### #16 A Yarn in `~/.yarn/bin` replaces the configured version

- [ ] Return the tool's executable directory from `activate_environment`

**Problem:** the tool lists `$HOME/.yarn/bin` and Node's `bin` as globals directories
(`tools/node-depman/src/package_manager.rs:199-207`, `:259-268`), and moon puts globals directories
ahead of the toolchain's executable directory (moon `toolchain_plugin.rs:232-237`). Yarn 1's install
script and `yarn global add` put binaries in `~/.yarn/bin`, and `corepack enable` puts a `yarn` shim
in Node's `bin`.

**Live:** a task's PATH listed `~/.yarn/bin` before `$PROTO_HOME/tools/yarn/1.22.22/shims`. With
Corepack enabled in Node's `bin`, `which -a yarn` listed Corepack's shim first. I didn't put a
`yarn` in `~/.yarn/bin`, which is in the user's home.

This is `BUN.md` #1 for Yarn.

**Change:** return Yarn's executable directory (`<tool_dir>/shims` for Yarn 1–5, and `<tool_dir>`
for Yarn 6 after #1) in `activate_environment`'s `paths`, which moon puts ahead of toolchain paths
(`augmented_command.rs:29-34`). This pairs with #11, in the same function.

### #17 Keep generated Yarn state out of Docker skeletons

- [ ] Exclude Yarn's install state and unplugged packages

**Problem:** the `.yarn/**/*` glob (`toolchains/javascript/src/tier1.rs:51`) copies all of `.yarn`
into both skeletons, from the workspace root and from each project (moon
`crates/app/src/commands/docker/scaffold.rs:76-124`, `:273`), regardless of `.gitignore`. That's
right for releases, plugins, patches and a zero-install cache, but it also copies what Yarn's docs
say to gitignore: `.yarn/install-state.gz`, `.yarn/unplugged` (packages extracted and built on the
host), and Yarn 6's `.yarn/ignore/`. The sources phase lays them over what `docker setup` built in
the image.

**Live:** both skeletons got `.yarn/install-state.gz` with Yarn 4, and
`.yarn/ignore/{install,manifests}` with Yarn 6.

I couldn't show a failure without a Docker daemon. The install state doesn't change on a no-op
install, so it doesn't break layer caching by itself. The risk is a native package built on the host
in `.yarn/unplugged`.

**Change:** add `define_docker_metadata` to the JavaScript toolchain, returning
`!.yarn/install-state.gz`, `!.yarn/unplugged/**/*`, `!.yarn/build-state.yml` and
`!.yarn/ignore/**/*` for Yarn, like moon's own `vendor_dir_name` exclusions
(`toolchain_plugin.rs:317-319`).

### #18 Detection names

- [ ] `config_file_globs` (`toolchains/node-depman/src/tier1.rs:44`): add `.yarnrc`, Yarn 1's config
      file, which `.yarnrc.*` doesn't match. The JavaScript toolchain already lists it
      (`toolchains/javascript/src/tier1.rs:52`).

## Not planned

- **Pruning Yarn's cache in `docker prune`:** `yarn workspaces focus --production` drops dev
  packages from `.pnp.cjs`, but keeps their archives in the cache (confirmed with a local cache), as
  Yarn documents. PnP reads packages from the cache, so cleaning it would break the image.
- **Hashing `patch:` dependencies:** `parse_manifest` skips them (`tier2.rs:721`), so editing a file
  in `.yarn/patches` changes no task hash unless the lockfile or the patch is a task input. The
  unpatched entry comes first in the lockfile, so resolving the inner range wouldn't help.
- **Reading `packageManager` to tell Yarn versions apart** (#3): it only matters before the first
  lockfile.

## Outside this repo

moon changes are tracked in [`MOON_COMPACT.md`](MOON_COMPACT.md). This audit added Yarn PnP to "Let
plugins report whether dependencies are installed" (#14), `yarn.plugins` to "Schedule
`SetupEnvironment` for toolchains without a dependencies root" (#10), "Don't fail every task on a
lockfile that can't be parsed" (#2), and the Yarn website docs.

**Yarn**

- [ ] zpm: `enableImmutableInstalls` stays `false` in CI (6.0.0-rc.23), unlike Yarn 2–4 (#9).

## Release

- `javascript_toolchain`: #2–#7, #8's lockfile fix, #9, #14, #15 and #17 change installs, lockfile
  parsing and project sync, so ship them as 1.4.0, with callouts for the dedupe and production
  commands (#3, #4), the Yarn 1 sync format (#5) and frozen CI installs (#9). #12 ships with
  `JAVASCRIPT.md` #15. It embeds no tool, so the proto 0.64 block on other toolchains
  (`MOON_COMPACT.md`, "Ship proto 0.64") doesn't apply: a `master` build loaded in moon 2.6.4.
- `node_depman_tool`: #1, #11 and #16 change which Yarn runs, so ship them as a minor (0.22.0).
- `node_depman_toolchain`: re-exports the tool, plus #8's `installArgs` description, #10's
  deprecation and #18, as a minor (1.2.0). It's blocked until moon ships proto 0.64, as it embeds a
  tool that requires it, and it's shared with `NPM.md`, `PNPM.md` and `NUB.md`, so coordinate one
  release.

## Handed to other plans

- **JAVASCRIPT.md:** besides aliases (your #14), `version_satisfies` (`config.rs:153-174`) is
  `false` for every prerelease, as version_spec follows semver (`>=2.0.0` doesn't match
  `6.0.0-rc.23`), so other package managers' prereleases take the old-version paths. #3 replaces the
  Yarn uses, so your #14's `latest` change doesn't affect Yarn.
- **JAVASCRIPT.md:** the install fingerprint only includes the JavaScript toolchain's config (moon
  `install_dependencies.rs:26-35`, `:141`), so changing `yarn.installArgs` (or npm's or pnpm's)
  doesn't re-run the install.
- **NODE.md:** confirmed live for Yarn: with Corepack enabled in Node's `bin`, a task's
  `which -a yarn` lists Corepack's shim ahead of `$PROTO_HOME/tools/yarn/<version>/shims` ("Outside
  this repo" says "from source, not reproduced").
- **BUN.md:** yes, Yarn's globals directories (`~/.yarn/bin`, Node's `bin`) can hold Yarn itself;
  see #16.
- **PNPM.md:** for #9, Yarn Switch (Yarn 6) reads only `packageManager`, not
  `devEngines.packageManager` (zpm `packages/zpm-switch/src/manifest.rs:115-125`), so keep writing
  `packageManager` for Yarn.
