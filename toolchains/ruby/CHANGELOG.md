# Changelog

## Unreleased

#### 💥 Breaking

- Commands now run `bundle` instead of `bundler`. proto only shims `bundle`, so `bundler` could
  resolve to the system Ruby's Bundler, like `/usr/bin/bundler` on macOS.
- `frozen` is now applied with `BUNDLE_FROZEN` on the install command, instead of being persisted
  into `.bundle/config`, so it no longer blocks `bundle add` and `bundle update` outside of moon.
  Bundler's local config takes precedence over environment variables, so checkouts set up by 0.1.x
  stay frozen until you run `bundle config unset --local frozen`.
- The `bin/` binstubs directory is now only added to `PATH` for tasks, and only when it exists. It
  was previously added for all commands, including other toolchains' commands, where binstubs like
  `bin/yarn` or `bin/vite` could shadow their tools.
- Task hashes no longer include `version` (moon already hashes it), `bundlePath` or `frozen`.
- Installs in CI are now frozen when a lockfile exists, as Bundler would otherwise rewrite an
  out-of-date lockfile. A lockfile that doesn't list the CI platform now fails the install, which
  `bundle lock --add-platform <platform>` fixes.
- Task hashes now include the locked version of every gem in the Gemfile, so they change once, and
  then after each `bundle update`.
- `bundlePath` is now only read from the workspace's toolchain config. moon checks for installed
  gems in it, so a project's override reinstalled them on every run. A project's value is now
  ignored with a warning.

#### 🚀 Updates

- Local gems in subdirectories of a Gemfile, like `gem "billing", path: "libs/billing"`, are now
  reported as members of its dependency root, so their projects get the root's install. Commands
  also set `PROTO_RUBY_VERSION` when `version` is configured, as moon doesn't set it for a shared
  root's commands.
- Tasks that run a gem executable which is only installed in the bundle, like `rspec`, now run it
  with `bundle exec`. A binstub of the same name in `bin/` takes precedence.
- Project graph inference now adds edges from gemspec dependencies, sets the gem name as the project
  alias, and finds `path` block gems by name.
- Lockfile versions with RubyGems segments, like `7.1.3.4` or `1.0.0.pre`, are now parsed, and git
  gems are hashed by their revision.
- Task detection now includes `rails`, `rspec` and `rubocop`, and project detection includes
  `Rakefile` and `.rspec`.
- Added `prune_docker` support. `moon docker prune` now persists the skipped groups with
  `bundle config set --local without`, as `bundle exec` fails on the missing gems otherwise, and
  runs `bundle clean` to remove their gems when `deleteVendorDirectories` is enabled.
- Docker scaffolding now copies `lib/**/version.rb`, which gemspecs created by `bundle gem` require,
  so `bundle install` no longer fails in the configs phase.
- Project graph inference now supports `gems.rb`, `path "<dir>" do` blocks, `gem` calls that span
  multiple lines, and `gemspec path:`.
- Panics now report their message, instead of an `unreachable` WASM trap error.

#### 🐞 Fixes

- Fixed `bundlePath` changes never being applied once `.bundle/config` exists. The `path` setting is
  now compared against the local config, and updated when it differs.
- Fixed `moon init` pre-filling an invalid `version` from a `.ruby-version` for another engine, like
  `jruby-9.4.5.0`.

#### ⚙️ Internal

- Replaced the line-based Gemfile and gemspec scanner with a [winnow](https://docs.rs/winnow)
  parser. It also finds gems in one-line blocks, like `group :test do gem "rspec" end`, local gems
  with an `if` or `unless` modifier, and dependencies in generated gemspecs, like `%q<rake>`.

## 0.1.2

#### 🚀 Updates

- Updated to support moon v2.6 release.

## 0.1.1

#### 🚀 Updates

- Updated to support moon v2.5 release.

## 0.1.0

#### 🚀 Updates

- Initial release of the Ruby toolchain plugin.
  - Tier 1: project/task detection (Gemfile, Gemfile.lock, gemspecs, .ruby-version).
  - Tier 2: dependency root location, environment setup (`bundle config`),
    `bundle install`, Gemfile/Gemfile.lock parsing, and `path:`-gem project
    graph inference.
  - Tier 3: Ruby installation via the proto `ruby_tool` plugin.
