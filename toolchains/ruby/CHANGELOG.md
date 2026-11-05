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

#### 🚀 Updates

- Added `prune_docker` support. `moon docker prune` now persists the skipped groups with
  `bundle config set --local without`, as `bundle exec` fails on the missing gems otherwise, and
  runs `bundle clean` to remove their gems when `deleteVendorDirectories` is enabled.
- Docker scaffolding now copies `lib/**/version.rb`, which gemspecs created by `bundle gem` require,
  so `bundle install` no longer fails in the configs phase.
- Project graph inference now supports `gems.rb`, `path "<dir>" do` blocks, `gem` calls that span
  multiple lines, and `gemspec path:`.

#### 🐞 Fixes

- Fixed `bundlePath` changes never being applied once `.bundle/config` exists. The `path` setting is
  now compared against the local config, and updated when it differs.
- Fixed `moon init` pre-filling an invalid `version` from a `.ruby-version` for another engine, like
  `jruby-9.4.5.0`.

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
