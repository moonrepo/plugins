pub mod download;
pub mod migrator;
pub mod project_graph;

pub use common::*;
use moon_pdk::VirtualPath;
use std::borrow::Cow;

pub fn format_virtual_path(path: &VirtualPath) -> Cow<'_, str> {
    if let Some(real) = path.real_path_string() {
        Cow::Owned(real)
    } else if let Some(rel) = path.without_prefix() {
        rel.to_string_lossy()
    } else if let Some(virt) = path.virtual_path_string() {
        Cow::Owned(virt)
    } else {
        Cow::Owned(path.to_string())
    }
}
