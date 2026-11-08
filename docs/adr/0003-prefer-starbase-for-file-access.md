---
status: accepted
date: 2026-11-07
applies-to: [backends/**, crates/**, extensions/**, toolchains/**, tools/**]
---

# Prefer `starbase_utils` for reading and writing files

## Context

Plugins read and write manifests, lockfiles and configs in users' repositories. `starbase_utils`
wraps these operations with errors that name the file involved, and with helpers like `write_file`,
which creates missing parent directories. Using it everywhere keeps error messages and behavior the
same across plugins.

It doesn't cover everything, and checking whether a path exists doesn't benefit from a wrapper, so
this is a preference rather than a requirement.

## Decision

1. Reading and writing files SHOULD use `starbase_utils`: `fs` for files and directories, and
   `json`, `toml` and `yaml` for those formats.
2. Path checks like `Path::exists()`, `Path::is_file()` and `Path::is_dir()` MAY be called directly.
3. `std::fs` MAY be used for operations that `starbase_utils` doesn't provide.

## Exceptions

None.

## Consequences

- File errors name the path, so users can tell which file failed to read or write.
- Plugins depend on `starbase_utils` behavior, such as `write_file` creating parent directories.
