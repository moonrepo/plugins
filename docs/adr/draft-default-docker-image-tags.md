---
status: proposed
date: 2026-11-18
applies-to: [crates/toolchain-common/**, toolchains/**]
---

# Default Docker images are published tags inside the configured version

## Context

`define_docker_metadata` returns the default image for `moon docker file`, built from the
toolchain's configured `version`. Seven toolchains did this (Bun, Deno, Go, Node.js, Python, Ruby
and Rust), each with its own copy of the same logic, mostly `to_partial_string()`. That drops the
operator, so every version bound was tagged by its version:

- A lower bound picked the oldest image allowed, like `python:3.10` for `>=3.10`.
- An upper bound picked an image outside the range, like `python:3.13` for `<3.13`, or one that
  doesn't exist, like `oven/bun:2` for `<2` (`BUN.md` #10).

The copies also disagreed on aliases: Node.js, Deno and Ruby passed any alias through as a tag,
while Go and Python dropped every alias. Checked against Docker Hub on 2026-11-18:

- `denoland/deno` only publishes full versions, like `2.9.8`, and `latest`, so every partial
  version, like `2.9` or `~2.9.0`, and `canary` named a tag that doesn't exist.
- `node` publishes `lts` and each LTS codename, like `jod`, but not `stable`, which proto's Node.js
  tool resolves, or `lts-jod`.
- `oven/bun` publishes `canary`, and `rustlang/rust` publishes `nightly`.

## Decision

1. `define_docker_metadata` MUST build its default image's tag with
   `toolchain_common::get_docker_image_tag`, except for the cases under Exceptions.
2. A default image MUST NOT name a version outside the configured version. When no published tag
   fits, like for a lower bound, an upper bound that excludes its version, or a range, the tag MUST
   be `latest`.
3. A toolchain MUST only map an alias, or `canary`, to a tag that the image publishes. A new mapping
   SHOULD be checked against the registry.

## Exceptions

- **Deno** only uses the helper for full versions, as `denoland/deno` has no partial tags. Anything
  else is `latest`.
- **Node.js** uses `lts` when no version is configured, as `latest` follows the Current release,
  which isn't an LTS for its first months. Pre-releases are `latest`, as `node` has no images for
  them.
- **Python** formats full versions itself, as Docker Hub uses CPython's form for pre-releases, like
  `3.15.0rc2`.
- **Rust** uses the `rustlang/rust:nightly` image for `canary`, `nightly` and dated nightlies.

## Consequences

- A fix to how versions become tags applies to every toolchain at once.
- `latest` for an upper bound, like `<2`, is still outside the range once a newer major ships.
- `^` and `~` keep their partial version, like `1.2` for `^1.2`, which is inside the range but
  isn't always the newest version it allows.
- Go pre-releases aren't mapped: `golang` tags them like `1.26rc1`, while the helper writes
  `1.26.0-rc.1`.
