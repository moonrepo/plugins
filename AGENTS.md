# AGENTS.md

Guidance for AI coding agents working in the **plugins** repository, which holds the official WASM
plugins for moon (toolchains and extensions) and proto (tools and backends).

## Repository layout

| Path           | Contents                                                                 |
| -------------- | ------------------------------------------------------------------------ |
| `tools/*`      | proto tool plugins, which resolve versions of a tool and install it      |
| `backends/*`   | proto backend plugins, which install tools from third-party registries   |
| `toolchains/*` | moon toolchain plugins, which integrate a language and its dependencies  |
| `extensions/*` | moon extension plugins, which extend moon with new functionality         |
| `crates/*`     | Libraries shared between plugins                                         |
| `docs/adr`     | Architecture decision records, see [below](#architecture-decisions)      |
| `docs/todo`    | Fix plans from plugin audits                                             |
| `scripts`      | CI job planning and release scripts                                      |

Every plugin is a moon project. Project names, like `go-toolchain`, are listed in
`.moon/workspace.yml`.

## Plugin structure

- Each plugin crate has a `wasm` feature, on by default, which also enables the `wasm` feature of
  the shared crates it uses. Everything that calls the WASM host sits behind it, both the plugin's
  exported functions and shared helpers like `enable_tracing`. Keep parsers and other pure logic
  outside it, so they can be tested without the host. That code imports types from
  `proto_pdk_api` or `moon_pdk_api`. The PDK crates are optional dependencies that only `wasm`
  enables, so using them outside the feature fails to compile
  ([ADR-0002](docs/adr/0002-wasm-feature-gates-host-code.md)).
- Tools and backends export proto's plugin API from `src/proto.rs`. Extensions export theirs from
  `src/<name>_ext.rs`.
- Toolchains split their exports by tier:
  - `tier1.rs`: registration, config, project syncing and Docker.
  - `tier2.rs`: the project graph, tasks, dependencies and environment setup.
  - `tier3.rs`: installing the tool, by re-exporting its proto tool crate (`pub use go_tool::*;`).
    A fix in a tool also needs a release of every toolchain that re-exports it.
- Code shared between plugins goes in `crates/`: `common` for all plugins, `tool-common`,
  `backend-common`, `toolchain-common` and `extension-common` for one plugin type, and
  `lang-*-common` for one language.

## Development workflow

Use `moon` for all Rust tasks. **Always run format, lint, and test before finishing a change.**

- `moon run <project>:build` - Builds the WASM plugin.
- `moon run <project>:check` - Checks the plugin without producing binaries.
- `moon run <project>:lint` - Lints the plugin with `clippy`, failing on any warning.
- `moon run <project>:test` - Runs all tests in the plugin with `cargo nextest`.
- `moon run root:format` - Checks the formatting of all Rust code with `rustfmt`.
- `moon run root:format-write` - Formats all Rust code with `rustfmt`.

Refer to `.moon/workspace.yml` for all available projects.

### Conventions

- **Shared target cache.** When running Cargo or any Just commands, the `$CARGO_TARGET_DIR`
  environment variable MUST be defined. If it's not provided by the shell, it should default to
  `~/.cargo/shared-target`.
- **No warnings.** Clippy runs with `-D warnings`; treat every warning as an error. When running
  clippy directly, pass `-- -D warnings` too.
- **Snapshots** use [`insta`](https://insta.rs/); review changes with `cargo insta review`.
- **Tests** run under [`nextest`](https://nexte.st/). Tests must pass with `STARBASE_TEST=true`.

### Testing

- Tests load the plugin's prebuilt release `.wasm`, which is why `test` depends on `build`. If a
  test shows old behavior, moon may have replayed a cached build, or another worktree may have
  overwritten the `.wasm` in the shared target dir. Rebuild it with
  `cargo build -p <crate> --target wasm32-wasip1 --release`, then test again.
- When running `cargo test` or `cargo nextest run` directly, pass `--no-default-features`.
  Otherwise the test binaries fail to link the WASM host imports. The `test` task already passes it.
- In agent sessions, unset `AI_AGENT` and `CLAUDECODE` before testing. Otherwise tool wrappers print
  a banner to stdout, and tests that parse JSON output fail with errors like
  `invalid type: map, expected a sequence`.

## Changelogs and releases

- Add an entry for every user-facing change to the plugin's `CHANGELOG.md`, under `## Unreleased`.
- Versions are bumped when a maintainer releases with `cargo release` (see
  [`docs/release.md`](docs/release.md)), so don't change them in `Cargo.toml`.

## Architecture decisions

Rules for how plugins are built live in [`docs/adr`](docs/adr) as architecture decision records
(ADRs). Accepted ADRs are binding. [ADR-0001](docs/adr/0001-record-architecture-decisions.md)
defines the process.

- Before changing a plugin, read every accepted ADR whose `applies-to` matches a path you're
  changing.
- If a change can't be made without breaking an ADR, stop and ask. Don't deviate silently, and don't
  edit the ADR to fit your change.
- If you make a decision that applies to more than one plugin, draft an ADR at
  `docs/adr/draft-<title>.md` with `status: proposed`, and mention it in your summary. Never set an
  ADR's status to `accepted`.
- Where code enforces a rule or is an exception to one, cite it in a comment, like `// ADR-0003.1`.

| ADR                                                          | Status   | Applies to    | Decision                                                                 |
| ------------------------------------------------------------ | -------- | ------------- | ------------------------------------------------------------------------ |
| [0001](docs/adr/0001-record-architecture-decisions.md)       | proposed | `docs/adr/**` | Cross-plugin decisions are ADRs, which only a maintainer accepts         |
| [0002](docs/adr/0002-wasm-feature-gates-host-code.md)        | accepted | All crates    | PDK crates are optional, and only used behind the `wasm` feature         |
| [0003](docs/adr/0003-prefer-starbase-for-file-access.md)     | accepted | All crates    | Prefer `starbase_utils` for reading and writing files                    |
| [draft](docs/adr/draft-cache-expensive-work-in-memory.md)    | proposed | All crates    | Cache expensive work in memory, and never fail on a var write            |
| [draft](docs/adr/draft-unit-tests-outside-wasm-feature.md)   | proposed | All crates    | Unit tests live outside the `wasm` feature, so the `test` task runs them |
| [draft](docs/adr/draft-ask-the-host-not-the-wasm-runtime.md) | proposed | All crates    | Ask the host for env vars, the OS, commands and output, not WASI         |
