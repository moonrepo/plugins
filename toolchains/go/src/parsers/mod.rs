//! Winnow grammars for Go's module files and `go version -m` output. Each one
//! turns its input into entries, which the `go_sum` and `go_work` modules and
//! the tier 2 functions map into their own types.

pub mod build_info;
mod common;
pub mod go_sum;
pub mod go_work;
