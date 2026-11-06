//! Winnow grammars for Go's module files. Each one turns a file into its
//! entries, which the `go_sum` and `go_work` modules map into their types.

mod common;
pub mod go_sum;
pub mod go_work;
