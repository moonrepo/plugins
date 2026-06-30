# Unreleased

#### 🚀 Updates

- Initial release of the Ruby toolchain plugin.
  - Tier 1: project/task detection (Gemfile, Gemfile.lock, gemspecs, .ruby-version).
  - Tier 2: dependency root location, environment setup (`bundle config`),
    `bundle install`, Gemfile/Gemfile.lock parsing, and `path:`-gem project
    graph inference.
  - Tier 3: Ruby installation via the proto `ruby_tool` plugin.
