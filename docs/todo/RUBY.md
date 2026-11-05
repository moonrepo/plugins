# Ruby toolchain fix plan

Fixes from the Ruby toolchain audit (2026-11-04, against moon v2.6.5 and proto 0.63.1). Covers
`toolchains/ruby` (`ruby_toolchain` 0.1.2, enabled as `unstable_ruby`) and `tools/ruby` (`ruby_tool`
0.3.0), which the toolchain re-exports as its tier 3 API. Item numbers (#1–#19) match the audit
findings.

## Testing

Build the WASM first. The test sandbox loads the prebuilt `.wasm`, so rebuild after every source
change; `moon run :build` can replay a stale cached build.

```bash
cargo build -p ruby_toolchain -p ruby_tool --target wasm32-wasip1 --release
```

```bash
cargo nextest run -p ruby_toolchain -p ruby_tool --no-default-features
```

In agent sessions, unset `AI_AGENT` and `CLAUDECODE` first, or wrapper banners can break JSON tests.

The `prune_docker` tests run real Bundler commands and skip themselves without a working `bundle`.
proto's shim needs a pinned Ruby for that, so run them with
`PROTO_RUBY_VERSION=<installed version>`.

Bundler behavior was checked with proto's Ruby 3.4.10 (Bundler 2.6.9) in a scratch app: a `Gemfile`
with `rainbow` plus `ansi` in a `test` group. Repeat that check for #3–#5, since those items depend
on Bundler itself rather than on plugin output.

## Decisions needed

| Decision                                                         | Recommendation                                                      |
| ---------------------------------------------------------------- | ------------------------------------------------------------------- |
| D1 (#11): frozen installs in CI by default?                      | Yes, when a lockfile exists, as the JavaScript toolchain does       |
| D2 (#12): run bare gem executables through `bundle exec`?        | Yes, only when the exe is in the bundle and not a binstub in `bin/` |
| D3 (#13, #14): support one shared Gemfile for many projects?     | Yes: report the root's local gems as members, and add gemspec edges |
| D4 (#15): feed gem versions to the task hasher?                  | Yes: every literal Gemfile gem, with lockfile versions normalized   |
| D5 (#16): build from source by default where no prebuilt exists? | Yes on macOS x64 and Linux musl; keep the error on Windows          |

## Phase 1: clear-cut fixes

### #1 Parse `.ruby-version` the way version managers do

- [x] Add `parse_version_file` to `tools/ruby`
- [x] Reuse it in `initialize_toolchain`
- [x] Tests

**Problem:** `detect_version_files` (`tools/ruby/src/proto.rs:38`) lists `.ruby-version`, but the
plugin has no `parse_version_file`, so proto parses the raw contents. RVM writes `ruby-3.3.5`, and
rbenv and chruby accept it. `UnresolvedVersionSpec` parses that as a version scoped to `ruby`, which
never matches a released version. Confirmed with proto 0.63.1: `ruby-3.3.5` gives
`Failed to resolve ruby-3.3.5 to a valid supported version for Ruby`, and so does `jruby-9.4.5.0`,
which other version managers would treat as a different engine. `initialize_toolchain`
(`toolchains/ruby/src/tier1.rs:62-75`) already strips `ruby-`, so moon's init and proto disagree on
the same file.

**Change:**

- Add a pure helper to `tools/ruby` (no `wasm` gate, so the toolchain can call it):
  - trim the contents and take the first non-empty line that doesn't start with `#`
  - strip a leading `ruby-`
  - return `None` for other engines (`jruby-`, `truffleruby-`, `mruby-`, `artichoke-` and so on),
    with a `host_log!` warning
- Implement `parse_version_file` with it. When it returns `None`, proto moves on to the next file.
- Replace the hand-rolled parsing in `initialize_toolchain` with the helper, so a JRuby pin no
  longer pre-fills an invalid `version`.

**Tests:** `tools/ruby/tests/versions_test.rs` covers `3.3.5`, `ruby-3.3.5`, `3.3`, `jruby-9.4.5.0`
and a commented file. `tier1_test.rs` covers a JRuby pin being skipped by init.

### #2 Run `bundle`, not `bundler`

- [x] Switch every emitted command
- [x] Update tests

**Problem:** every command the plugin emits runs `bundler` (`tier2.rs:48`, `:64`, `:79`). proto
creates shims only for the executables in `locate_executables` (`bundle`, `bundle-3`, `bundle-3.4`;
there's no `bundler` shim), and macOS ships `/usr/bin/bundler` for the system Ruby. When moon runs a
plugin command without the toolchain's bin directory on PATH (see #118 under "Outside this repo"),
`bundler` resolves to the system Ruby's Bundler instead of proto's.

**Change:** use `bundle` everywhere. It's the documented command name, and proto shims it.

**Found while implementing:** with no Ruby pinned, proto's `bundle` shim runs the system `ruby` with
the arguments instead (`bundle -v` prints `ruby 2.6.10…` on macOS), so a plugin command that relies
on the shim still needs a pinned version. #13's `PROTO_RUBY_VERSION` covers that.

### #3 Keep `.bundle/config` in step instead of skipping once it exists

- [x] Read the local config
- [x] Emit `path` only when it differs
- [x] Tests

**Problem:** `setup_environment` returns early when `.bundle/config` exists (`tier2.rs:39`).

- Changing `bundlePath` after the first setup is never applied, so gems stay in the old directory
  while `vendor_dir_name` reports the new one.
- Any other local setting (`bundle config set --local jobs 4`, `without`, and so on) creates the
  file, after which `path` is never set.

**Change:** read `<root>/.bundle/config`, which is flat YAML with one `BUNDLE_<KEY>: "<value>"` per
line (confirmed). Emit `bundle config set --local path <bundlePath>` only when `BUNDLE_PATH`
differs. The command list then still changes with disk state, which moon's setup fingerprint relies
on.

When `BUNDLE_APP_CONFIG` is set (`get_host_env_var`), Bundler keeps local config there instead. The
official `ruby` image sets it to `/usr/local/bundle`, which is outside the workspace, so always emit
the command in that case.

**Tests** (`tier2_test.rs`): a config with the same path emits nothing; a different path emits the
command; a config holding only `BUNDLE_JOBS` emits the command.

### #4 Make `frozen` an install-time setting

- [x] Move it to the install command's env
- [x] Changelog note

**Problem:** `frozen: true` persists `BUNDLE_FROZEN` into `.bundle/config` (`tier2.rs:62-67`). That
blocks `bundle add` and `bundle update` outside moon in that checkout, and setting `frozen: false`
later never removes it.

**Change:** drop the setup command and set `BUNDLE_FROZEN=true` in `install_dependencies`'s command
env when `frozen` is on.

**Changelog:** Bundler's local config takes precedence over env vars (confirmed with
`bundle config get frozen`). Checkouts set up by 0.1.x therefore stay frozen until someone runs
`bundle config unset --local frozen`.

### #5 Production installs must persist `without`, and docker prune should clean

- [x] Implement `prune_docker`
- [x] Share the group list with `install_dependencies`
- [x] Tests or a documented manual check

**Problem:** moon's `docker prune` calls `prune_docker` and then `install_dependencies` with
`production: true` (moon `crates/app/src/commands/docker/prune.rs:181`). The plugin has no
`prune_docker`, and the production install only sets `BUNDLE_WITHOUT` in that one command's env
(`tier2.rs:86-94`). This was confirmed with Bundler 2.6.9:

- `BUNDLE_WITHOUT=test bundle install`, then a plain `bundle exec ruby -e 'puts :ok'`, fails with
  `Bundler::GemNotFound: Could not find ansi-1.6.0 in locally installed gems`. The image's runtime
  `bundle exec` breaks the same way.
- After `bundle config set --local without test`, the same `bundle exec` works.
- After a full install, `bundle clean` with `without` persisted removes only the excluded gems, so
  native gems aren't rebuilt.
- Without a prune step, dev and test gems already in `vendor/bundle` also stay in the image.

**Change:** add `prune_docker`, which runs at `input.root` through `exec`, as the Rust toolchain's
prune does:

1. `bundle config set --local without <groups>`, using the `productionWithoutGroups` /
   `development:test` logic already in `install_dependencies` (factor it into one helper).
2. If `docker_config.delete_vendor_directories` is set, `bundle clean`.

Report `.bundle/config` and the bundle directory in `changed_files`. Keep the `BUNDLE_WITHOUT` env
on the production install; it does no harm.

**Tests:** the `exec` calls need a real Bundler, so the `prune_docker` tests in `tier1_test.rs` skip
themselves unless `bundle -v` prints Bundler's version. Locally, run them with Ruby pinned
(`PROTO_RUBY_VERSION=3.4.10`); GitHub's runners ship a system Ruby with Bundler.

### #6 Docker scaffold: copy what `bundle install` needs

- [x] Extend `scaffold_globs`
- [x] Test

**Problem:** the `bundle gem` template's gemspec starts with
`require_relative "lib/<name>/version"`. In the configs phase only `*.gemspec` is copied, so
`bundle install` fails with a `LoadError` whenever the Gemfile uses `gemspec` or a `path:` gem built
from that template.

**Change:** add `lib/**/version.rb`. moon already adds the toolchain's config file globs, manifests
and lockfiles (moon `crates/toolchain-plugin/src/toolchain_plugin.rs:313-315`), so `gems.rb` and
`gems.locked` were covered, and it applies the globs from each project root
(`crates/app/src/commands/docker/scaffold.rs:146`). The plugin's own list is now only this glob.

**Tests:** `scaffolds_gem_version_files` in `tier1_test.rs`.

### #7 Put binstubs on PATH for tasks only

- [x] Replace `extend_command` with `extend_task_command` and `extend_task_script`
- [x] Only add `bin/` when it exists
- [x] Tests

**Problem:** `extend_command` (`tier2.rs:103-119`) prepends `<root>/bin` for every command moon
augments, not just Ruby tasks. For workspace-level commands, which have no project, moon runs every
configured toolchain's `extend_command` (moon
`crates/process-augment/src/augmented_command.rs:126`). App binstubs therefore shadow other
toolchains' tools; for example, vite_ruby's `bin/vite` and the `bin/yarn` from Rails 5.1–6.1 shadow
the JavaScript toolchain's commands. After #2, Rails' `bin/bundle` binstub would also wrap the
plugin's own `bundle install`. The directory is also added when it doesn't exist.

**Change:** move the logic to `extend_task_command` and `extend_task_script`, locating the root from
the project fragment's source, and push `bin/` only if it exists.

**Tests:** replace the `extend_command` tests in `tier2_test.rs` with task command and task script
versions, plus a case with no `bin/` directory.

### #8 Hash only what moon doesn't already hash

- [x] Rework `hash_task_contents`
- [x] Tests

**Problem:** `hash_task_contents` (`tier2.rs:295-311`) hashes `version`, `bundlePath` and `frozen`.
moon already adds the project or workspace `version` to the task fingerprint (moon
`crates/task-hasher/src/task_hashing.rs:191-208`). `bundlePath` and `frozen` don't change what a
task produces, yet toggling either invalidates every Ruby task. When `version` is unset, which is
common in Ruby repos that pin only `.ruby-version`, the Ruby that actually runs (picked by proto
shims, rbenv or chruby) isn't hashed at all.

**Change:** return no contents when `version` is set. Otherwise, hash the contents of the nearest
`.ruby-version`, walking from the project root up to the workspace root, after #1's parsing.

**Changelog:** this changes every Ruby task hash once.

**Tests:** replace `includes_resolved_version` with three cases: `version` set gives no contents;
unset with a `.ruby-version` gives that version; unset with no file gives no contents.

### #9 Gemfile scanner gaps

- [x] `path "<dir>" do … end` blocks
- [x] Multi-line `gem` calls
- [x] `gemspec path: "<dir>"`
- [x] Unit tests in `gemfile.rs`

**Problem:** `parse_path_gems` (`gemfile.rs:16-63`) reads one line at a time and only understands
`gem …, path:`. It misses three common ways of declaring local gems:

- Path blocks, as used by component-based Rails apps: `path "components" do gem "admin_ui" end`. The
  block is treated as a plain block (`gemfile.rs:31-38`), and the inner `gem` has no `path:`, so
  it's skipped.
- `gem "billing",` followed by `path: "../billing"` on the next line. The first line has no `path:`,
  and the second isn't a `gem` call.
- `gemspec path: "../other"`.

**Change:**

- Join a line ending in `,` or `\` with the next line before parsing.
- Treat `path "<dir>" do` as a block whose gems resolve to `<dir>/<name>`. Bundler actually finds
  them by gemspec name under `<dir>`, so #14's name map can replace this guess later.
- Parse `gemspec path:` as a local gem. A bare `gemspec` is the project itself, so skip it.
- Push a block for `do |…|` endings and `def` lines, so their `end` doesn't pop an enclosing group.

### #10 Small fixes

- [x] `extend_project_graph` reads only `Gemfile` (`tier2.rs:229-232`). Also read `gems.rb`, as
      `parse_manifest` does.
- [x] `tools/ruby/README.md` says the plugin has no configuration. Document `use-latest-build`.

## Phase 2: after decisions

### #11 Frozen installs in CI (D1)

- [ ] Set `BUNDLE_FROZEN=true` in CI
- [ ] Tests

**Problem:** Bundler, unlike npm or pnpm, doesn't freeze the lockfile in CI, so `bundle install` in
CI can silently rewrite `Gemfile.lock`. The JavaScript toolchain already uses frozen installs when
`env.ci` is set (`toolchains/javascript/src/tier2.rs:428-470`).

**Recommended:** when `get_host_environment()?.ci` is set and the root has `Gemfile.lock` or
`gems.locked`, set `BUNDLE_FROZEN=true` on the install command (#4). A lockfile that doesn't list
the CI platform then fails with Bundler's `bundle lock --add-platform` hint. That's the right
failure, but it needs a changelog callout.

### #12 Bare gem executables (D2)

- [ ] Rewrite to `bundle exec` when the exe is only in the bundle
- [ ] Tests

**Problem:** with gems installed into `bundlePath`, gem executables live in
`<bundlePath>/ruby/<abi>/bin`, which isn't on PATH. A task with `command: rspec` or `rubocop` either
fails or runs an unrelated global gem. moon's own Ruby task examples (`rake build`) only work when
the Rakefile loads no gems from the bundle.

**Recommended:** in `extend_task_command`, rewrite `X` to `bundle exec X` when
`<root>/<bundlePath>/ruby/*/bin/X` exists and `<root>/bin/X` doesn't (binstubs from #7 win). Skip
`ruby`, `bundle`, `gem` and any command containing a path separator. This mirrors the Rust
toolchain's `cargo X` rewrite.

**Alternative:** document `bundle exec` and `bundle binstubs`, and change nothing.

### #13 Projects under a shared Gemfile get no install (D3)

- [ ] Report members from the root Gemfile
- [ ] Pin the Ruby version on emitted commands
- [ ] Fixture and tests

**Problem:** `locate_dependencies_root` (`tier2.rs:17-29`) never reports `members`. moon's
`in_dependencies_workspace` (`crates/toolchain-plugin/src/toolchain_plugin.rs:164`) then only
accepts the root itself. A project without its own Gemfile, such as `libs/billing` under a root
Gemfile with `gem "billing", path: "libs/billing"`, gets no setup or install action (moon
`crates/action-graph/src/action_graph_builder.rs:422-437`). Its tasks run against a bundle moon
never installed.

**Recommended:**

- Report `members` as the directories of the root Gemfile's local gems, from #9, relative to the
  root. These are the gems Bundler actually treats as local, and every project resolving to the root
  gets the same list, so moon collapses them into one install. Report `None` when there are none.
- Members make the root's role a workspace root, so moon runs its commands with no project. That
  hits #118 below: no toolchain PATH or version env. When `version` is set, add
  `PROTO_RUBY_VERSION=<version>` to the env of every emitted command, so proto's `bundle` shim (#2)
  picks the configured Ruby.

**Alternative:** `members: ["**/*"]`. It's simpler, but moon then walks the whole tree for manifests
on every install fingerprint (moon `crates/actions/src/actions/install_dependencies.rs:333`).

**Not covered:** a nested project that uses the root bundle but isn't declared in the root Gemfile.

**Tests:** fixture `__fixtures__/shared-gemfile` (root Gemfile with path gems, no Gemfile in the
libraries) for `locate_dependencies_root`.

### #14 Edges from gemspec dependencies (D3)

- [ ] Map gem names to projects
- [ ] Link `add_dependency` and `add_development_dependency`
- [ ] Optionally set the gem name as the project alias
- [ ] Tests

**Problem:** in the shared-Gemfile layout, dependencies between libraries live in gemspecs
(`spec.add_dependency "billing"`), not Gemfiles. `extend_project_graph` only reads each project's
Gemfile (`tier2.rs:228-267`), so those projects have no edges, which breaks `^:` task deps and
affected detection.

**Recommended:** scan `<project>/*.gemspec` line by line, using the same approach as the Gemfile
scanner:

- The gem name is the literal `<var>.name = "<name>"`, falling back to the file stem.
- Link `add_dependency` and `add_runtime_dependency` (production), and `add_development_dependency`
  (development), when the first argument is a literal naming another project's gem.
- Push gemspecs into `input_files`.
- Optionally return the gem name as the project `alias`, as JavaScript does with package names.

### #15 Gem versions in task hashes (D4)

- [ ] `parse_manifest`: report every literal gem
- [ ] `parse_lock`: normalize RubyGems versions
- [ ] Tests

**Problem:** moon's task hasher only records manifest dependencies that have a version requirement,
resolved through the lockfile (moon `task_hashing.rs:420-470`). `parse_manifest`
(`tier2.rs:184-211`) only returns `path:` gems, which have no version, so a `bundle update` never
changes a task hash unless the lockfile is itself a task input. That isn't the case for a root
lockfile (#13).

`parse_lock` also loses many versions. `VersionSpec::parse` rejects RubyGems forms (confirmed with
version_spec 0.11.7): `7.1.3.4` (every Rails patch release), `3.1`, `1`, `1.0.0.pre` and
`8.1.0.alpha` all become `version: None`. rubund itself parses them, and checksums with platforms,
correctly.

**Recommended:**

- `parse_manifest` reports each literal `gem "<name>"` with requirement `*`, by group scope. That
  makes the hasher record the locked checksum or version. Translating `~>` isn't worth it, since the
  lockfile is the source of truth.
- `parse_lock` normalizes versions before parsing:
  - pad to three numeric segments
  - put extra numeric segments into build metadata (`7.1.3.4` → `7.1.3+4`)
  - turn the first alphabetic segment and everything after it into a prerelease (`1.0.0.pre` →
    `1.0.0-pre`)

**Not covered:** `*` may not match prerelease gems, which would fall back to recording `*`. Probe
this first.

### #16 Build from source where no prebuilt exists (D5)

- [ ] Choose the default strategy by host
- [ ] Tests

**Problem:** jdx/ruby prebuilts exist only for macOS arm64 and glibc Linux (x64 and arm64). The
registry tags them `libc: gnu`, so musl hosts are correctly not matched. On macOS x64 and Alpine,
`default_install_strategy: DownloadPrebuilt` (`tools/ruby/src/proto.rs:22`) always fails with "No
pre-built available". moon's tier 3 setup installs with the tool's `default_install_strategy` and
has no `--build` equivalent (moon `crates/toolchain-plugin/src/toolchain_plugin.rs:676`), so
`unstable_ruby.version` fails there.

**Recommended:** in `register_tool`, return `BuildFromSource` when the host is macOS x64 or Linux
musl (`get_host_environment`). moon picks this up with no moon changes. Leave Windows as it is;
ruby-build doesn't support it.

## Phase 3: lower priority

### #17 Verify prebuilt checksums

- [ ] Emit `checksum` once the registry has one

The registry's ruby artifacts have no checksum file, so downloads aren't verified. GitHub's API
exposes a `sha256:` digest for every jdx/ruby asset, and releases include an `.intoto.jsonl`
provenance file. When the registry records the digest (see "Outside this repo"), set
`DownloadPrebuiltOutput.checksum`.

### #18 Detection names

- [ ] `exe_names` (`tier1.rs:41-48`): add `rails`, `rspec` and `rubocop`.
- [ ] `config_file_globs` (`tier1.rs:25-29`): add `Rakefile` and `.rspec`.

### #19 Inherited `GEM_HOME`

- [ ] Probe before changing anything

chruby and RVM export `GEM_HOME` and `GEM_PATH` for their active Ruby, and moon tasks inherit them
while running proto's Ruby. `BUNDLE_PATH` isolates the bundle itself, but Bundler's own version
switch and `gem install` still write to the other Ruby's gem directory. Reproduce it with chruby
active before deciding to unset these variables.

## Not planned

- **Syncing `.ruby-version` from `version`:** Node's `syncVersionManagerConfig` was deprecated
  because it never worked, and Ruby repos usually treat `.ruby-version` as the source of truth,
  which proto reads once #1 lands. Revisit if users ask for it.
- **Windows:** RubyInstaller2 publishes portable archives, but supporting them is a feature, not an
  integration fix.

## Outside this repo

- [ ] moon (#118, still live): `AugmentedCommand::get_toolchain_ids` returns no toolchains for
      plugin commands with no project, so workspace-scoped setup and install commands get no
      toolchain PATH or version env. #13's `PROTO_RUBY_VERSION` is a plugin-side mitigation.
- [ ] moon `crates/toolchain-plugin/src/toolchain_plugin.rs:375`: `detect_task_usage` compares the
      whole command, so `bin/rails` and `./bin/rspec` are never detected as Ruby tasks. Match the
      file name when the command is a relative path. The proto-binary branch is also broken (see
      `RUST.md`).
- [ ] moon `website/docs/__partials__/setup-toolchain/ruby/tier3.mdx` says tier 3 requires building
      Ruby from source, which hasn't been true since `ruby_tool` 0.2.9. The `create-task/ruby`
      partials should use `bundle exec` or binstubs (#12).
- [ ] registry.moonrepo.app: store GitHub's sha256 digest for each jdx/ruby artifact (#17).
- [ ] proto: with no version pinned, the `bundle` shim (a secondary executable) runs the system
      `ruby` with the shim's arguments, instead of the system `bundle` or an error, as found while
      implementing #2.

## Release

- `ruby_toolchain`: #2, #4, #5, #7 and #8 change which binary or settings run, and change task
  hashes, so ship Phase 1 as 0.2.0 with changelog callouts (the toolchain is unstable and pre-1.0).
  #11–#15 can follow in 0.2.x or 0.3.0.
- `ruby_tool`: #1 is a patch (0.3.1), and #16 is a minor (0.4.0). The toolchain re-exports
  `ruby_tool`, so tool fixes also need a toolchain release.
