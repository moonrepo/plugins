---
status: proposed
date: 2026-11-07
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Cache expensive work in memory

## Context

moon and proto call plugin functions many times in one run, often with the same inputs, and every
call into the host has a cost. Plugins already cache some values in Extism vars: the asdf backend
caches the CPU count, the Deno, Go and Rust toolchains cache bin directories, and the JavaScript
toolchain caches workspace catalogs and members.

Extism limits the vars of a plugin instance to 1 MiB in total, by default. Past it, `var::set` fails
with "Variable store is full", which fails the plugin call when the error is propagated with `?`.
warpgate raises the default to 16 MiB in `inject_default_manifest_config`, which moon, proto and
their test sandboxes all use. moon and proto releases built with an older warpgate keep the 1 MiB
limit.

Each `var::get` also copies the value from the host into the plugin, where it's parsed again, so a
cached value costs a copy and a parse on every read.

`warpgate_api` v0.21, released with proto v0.64, adds `create_var_key` and
`create_var_key_with_hasher`. They build a key from a name and a 64-bit hash of the inputs, like
`workspace-catalogs:1a2b3c4d5e6f7a8b`, and are re-exported by `proto_pdk_api` and `moon_pdk_api`.

## Decision

1. Expensive work that a plugin needs more than once SHOULD be cached in memory. Expensive work is
   any call into the host, such as running a command, sending an HTTP request, or reading files or
   environment variables, and any computation of similar cost.
2. Within one plugin function call, the result SHOULD be computed once and passed to the code that
   needs it, rather than computed again.
3. Across plugin function calls, the result SHOULD be cached in an Extism var, with `var::get` and
   `var::set`.
4. A cache key MUST include every input the value depends on that can differ between calls in the
   same process, such as the workspace root or the tool version, so a value is never reused for a
   different input. Inputs that are fixed for the life of the process, like host environment
   variables and the home directory, MAY be left out.
5. Cache keys SHOULD be created with `create_var_key`, which hashes inputs that implement `Hash`,
   or with `create_var_key_with_hasher`, which passes the hasher to a closure for inputs that don't.
6. A var SHOULD only hold a value that's cheaper to copy and parse than to compute again.
7. A failure to write a var MUST NOT fail the plugin call. The plugin logs the error and continues
   without the cache. Plugins can't assume the 16 MiB limit, because older hosts keep 1 MiB.

## Exceptions

None.

## Consequences

- Repeated calls in one run avoid repeating host calls.
- A var lives as long as the plugin instance, which is usually one moon or proto process. A value
  can go stale within that process if something changes what it depends on, such as a command that
  installs a binary.
- On hosts with the 1 MiB limit, large values fail to cache, and the plugin computes them again on
  every call.
