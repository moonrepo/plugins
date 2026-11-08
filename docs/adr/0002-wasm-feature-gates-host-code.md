---
status: accepted
date: 2026-11-07
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Code that uses the WASM host is behind the `wasm` feature

## Context

Plugins are also compiled natively, without the `wasm` feature, for unit and integration tests.
Extism's host functions only exist inside the WASM runtime, so a native build that links code
referencing them fails with undefined symbol errors. Other linkers usually drop unused references,
but MSVC doesn't, and rustc can pull them into objects that native code links, because it shares
generic instances across crates and merges small codegen units. This failed the Windows checks for
several plugins (25b72962), and was fixed for `crates/common` by putting its host-calling helpers
behind a `wasm` feature (2dc95ecc).

Gating code by hand is easy to get wrong, and a mistake only shows up as a link error on Windows.
Making the PDK crates optional turns a mistake into a compile error on every platform.

## Decision

1. `extism-pdk`, `proto_pdk` and `moon_pdk` MUST be optional dependencies, enabled only by the
   crate's `wasm` feature, for example `wasm = ["dep:extism-pdk", "dep:proto_pdk"]`.
2. A crate's `wasm` feature MUST enable the `wasm` feature of each workspace crate it depends on that
   has one, such as `tool_common/wasm` or a re-exported tool crate's `go_tool/wasm`.
3. Code that uses those crates MUST be behind `#[cfg(feature = "wasm")]`. This includes
   `#[plugin_fn]` exports, `#[host_fn]` declarations, host macros like `host_log!` and
   `exec_command!`, and helpers that call the host.
4. Whole modules SHOULD be gated, by putting the attribute on the `mod` declaration. Individual items
   MAY be gated instead, when the rest of the module is pure logic, such as a parser that logs a
   warning through the host.
5. Code outside the feature MUST take types from `proto_pdk_api` or `moon_pdk_api`. Both re-export
   `warpgate_api`, which includes `AnyResult`, `VirtualPath` and the `anyhow!` macro.

## Exceptions

None.

## Consequences

- A violation fails to compile in any native build without the feature, including the `test` task,
  instead of failing only when linking on Windows.
- Parsers and other pure logic stay testable natively, without the WASM host.
- Each crate names the PDK crates twice in its `Cargo.toml`: once as dependencies, and again in its
  `wasm` feature.
