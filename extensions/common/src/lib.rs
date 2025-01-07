pub mod download;
pub mod migrator;
pub mod project_graph;

use moon_pdk::VirtualPath;
use std::borrow::Cow;

pub fn format_virtual_path(path: &VirtualPath) -> Cow<'_, str> {
    if let Some(real) = path.real_path() {
        Cow::Owned(real.to_string_lossy().into_owned())
    } else if let Some(rel) = path.without_prefix() {
        rel.to_string_lossy()
    } else if let Some(virt) = path.virtual_path() {
        Cow::Owned(virt.to_string_lossy().into_owned())
    } else {
        path.to_string_lossy()
    }
}
