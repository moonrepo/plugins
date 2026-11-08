---
status: proposed
date: 2026-11-07
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Unit tests live outside the `wasm` feature

## Context

The `test` task runs `cargo nextest run --no-default-features`, and no other task runs tests. A unit
test in code behind the `wasm` feature isn't compiled in that build, so it never runs, locally or in
CI. `check` still compiles it, so it looks like coverage when it isn't.

When this was found, 48 unit tests were in this state: 18 in the JavaScript toolchain's `lockfiles`
module, and 30 in the Java tool's `config` and `foojay` modules. They all passed when run by hand.
The Kotlin tool already avoided the problem by gating its modules with
`#[cfg(any(feature = "wasm", test))]`.

## Decision

1. `#[test]` functions MUST NOT be behind the `wasm` feature, so that the `test` task runs them.
2. Logic that needs tests but sits next to host calls SHOULD be moved out of the feature, either by
   gating only the items that call the host (ADR-0002.4), or by splitting it into its own module.
3. A module that only code behind the feature uses, but that has tests, SHOULD be gated with
   `#[cfg(any(feature = "wasm", test))]`. Native test builds then compile it and run its tests, and
   other native builds leave it out.
4. When a test build reports code in that module as dead, because only code behind the feature uses
   it, the module SHOULD also allow dead code in builds without the feature:

   ```rust
   #[cfg(any(feature = "wasm", test))]
   #[cfg_attr(not(feature = "wasm"), allow(dead_code, unused_imports))]
   mod lockfiles;
   ```

## Exceptions

None.

## Consequences

- Every unit test runs in the `test` task, locally and in CI.
- A module gated with `any(feature = "wasm", test)` can't use the PDK crates, because they're
  optional (ADR-0002.1), so its test build would fail to compile. Host calls in it need their own
  `#[cfg(feature = "wasm")]`.
- Testing logic sometimes means moving it away from the host calls it was written next to.
- In modules that allow dead code without the feature, truly unused code is only reported by builds
  with the feature, like the `lint` task's clippy run.
