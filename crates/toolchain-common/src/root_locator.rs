use moon_pdk_api::{AnyResult, VirtualPath};
use rustc_hash::FxHashMap;

/// Locates dependencies roots by traversing upwards from many starting
/// directories, which typically share ancestors (like projects in the
/// same workspace), so each file is only checked for existence once.
#[derive(Default)]
pub struct RootLocator {
    files: FxHashMap<VirtualPath, bool>,
}

impl RootLocator {
    /// Return true if the file exists, checking the file system only once.
    pub fn exists(&mut self, path: VirtualPath) -> bool {
        *self
            .files
            .entry(path)
            .or_insert_with_key(|path| path.exists())
    }

    /// Return true if any of the file names exist in the directory.
    pub fn exists_any(&mut self, dir: &VirtualPath, file_names: &[&str]) -> bool {
        file_names
            .iter()
            .any(|file_name| self.exists(dir.join(file_name)))
    }

    /// Locate the closest directory, starting from and including the starting
    /// directory, that contains any of the file names.
    pub fn locate(
        &mut self,
        starting_dir: &VirtualPath,
        file_names: &[&str],
    ) -> Option<VirtualPath> {
        let mut current_dir = Some(starting_dir.to_owned());

        while let Some(dir) = current_dir {
            if self.exists_any(&dir, file_names) {
                return Some(dir);
            }

            current_dir = dir.parent();
        }

        None
    }

    /// Like [`RootLocator::locate`], but also runs the check on each directory
    /// that contains any of the file names (once per directory), and only stops
    /// traversing when the check returns a value.
    pub fn locate_with_check<T>(
        &mut self,
        starting_dir: &VirtualPath,
        file_names: &[&str],
        mut check: impl FnMut(&VirtualPath) -> AnyResult<Option<T>>,
    ) -> AnyResult<Option<(VirtualPath, T)>> {
        let mut current_dir = Some(starting_dir.to_owned());

        while let Some(dir) = current_dir {
            if self.exists_any(&dir, file_names)
                && let Some(value) = check(&dir)?
            {
                return Ok(Some((dir, value)));
            }

            current_dir = dir.parent();
        }

        Ok(None)
    }
}

/// Values, like parsed manifests, that are loaded once per directory.
pub struct DirCache<T> {
    values: FxHashMap<VirtualPath, T>,
}

impl<T> Default for DirCache<T> {
    fn default() -> Self {
        Self {
            values: FxHashMap::default(),
        }
    }
}

impl<T> DirCache<T> {
    /// Return the value of the directory, loading it if not cached.
    pub fn get_or_load(
        &mut self,
        dir: &VirtualPath,
        load: impl FnOnce(&VirtualPath) -> AnyResult<T>,
    ) -> AnyResult<&T> {
        if !self.values.contains_key(dir) {
            let value = load(dir)?;

            self.values.insert(dir.to_owned(), value);
        }

        Ok(&self.values[dir])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;

    #[test]
    fn locates_closest_dir_with_any_file() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a.lock", "");
        sandbox.create_file("one/b.lock", "");

        let mut locator = RootLocator::default();
        let root = VirtualPath::new(sandbox.path());

        assert_eq!(
            locator.locate(&root.join("one/two"), &["a.lock", "b.lock"]),
            Some(root.join("one"))
        );
        assert_eq!(
            locator.locate(&root.join("other"), &["a.lock", "b.lock"]),
            Some(root.clone())
        );
        assert_eq!(locator.locate(&root.join("one/two"), &["c.lock"]), None);
    }

    #[test]
    fn checks_each_file_once() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("one/a.lock", "");

        let mut locator = RootLocator::default();
        let root = VirtualPath::new(sandbox.path());

        assert!(locator.locate(&root.join("one/two"), &["a.lock"]).is_some());

        std::fs::remove_file(sandbox.path().join("one/a.lock")).unwrap();

        assert_eq!(
            locator.locate(&root.join("one/three"), &["a.lock"]),
            Some(root.join("one"))
        );
    }

    #[test]
    fn checks_each_dir_once_and_continues_upwards() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("a.toml", "");
        sandbox.create_file("one/a.toml", "");
        sandbox.create_file("one/b.toml", "");

        let mut locator = RootLocator::default();
        let root = VirtualPath::new(sandbox.path());
        let mut checked = vec![];

        let result = locator
            .locate_with_check(&root.join("one/two"), &["a.toml", "b.toml"], |dir| {
                checked.push(dir.to_owned());

                Ok((dir == &root).then_some("found"))
            })
            .unwrap();

        assert_eq!(result, Some((root.clone(), "found")));
        assert_eq!(checked, [root.join("one"), root.clone()]);
    }

    #[test]
    fn loads_each_dir_once() {
        let mut cache = DirCache::default();
        let dir = VirtualPath::new("/workspace");
        let mut loads = 0;

        for _ in 0..2 {
            let value = cache
                .get_or_load(&dir, |_| {
                    loads += 1;

                    Ok(loads)
                })
                .unwrap();

            assert_eq!(*value, 1);
        }

        assert_eq!(loads, 1);
    }
}
