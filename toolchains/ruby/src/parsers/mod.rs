//! Winnow grammars for Ruby's dependency files. Each one turns a file into
//! its statements, which the `gemfile` module walks for gems.

pub mod gemfile;
