// Tier 3 (download & install Ruby) is delegated entirely to the proto `ruby_tool`
// plugin, which provides register_tool, load_versions, detect_version_files,
// build_instructions, and locate_executables. See tools/ruby.
pub use ruby_tool::*;
