// `go.work`

use moon_pdk::AnyResult;

#[derive(Debug, Default)]
pub struct GoWork {
    pub modules: Vec<String>,
    pub version: Option<String>,
}

impl GoWork {
    pub fn add_module(&mut self, path: &str) {
        if path.starts_with("..") {
            // Ignore since we only support child members
        } else {
            self.modules.push(path.trim_start_matches("./").into());
        }
    }

    // https://go.dev/ref/mod#go-work-file
    pub fn parse(content: impl AsRef<str>) -> AnyResult<Self> {
        let mut work = Self::default();
        let mut in_use_block = false;
        let mut in_replace_block = false;

        for mut line in content.as_ref().lines() {
            if line.starts_with("//") {
                continue;
            } else if let Some(index) = line.find("//") {
                line = &line[0..index];
            }

            // go 1.2.3
            if let Some(version) = line.strip_prefix("go ") {
                work.version = Some(version.into());
                continue;
            }

            // use (), replace ()
            if line.starts_with("use (") {
                in_use_block = true;
            } else if line.starts_with("replace (") {
                in_replace_block = true;
            } else if line.trim() == ")" {
                if in_use_block {
                    in_use_block = false;
                } else if in_replace_block {
                    in_replace_block = false;
                }
            }

            // use <path>
            if !in_use_block {
                if let Some(path) = line.strip_prefix("use ") {
                    work.add_module(path);
                    continue;
                }
            }

            // replace <path>
            if !in_replace_block && line.starts_with("replace ") {
                // Ignore for now!
                continue;
            }

            // <path>
            if in_use_block {
                if let Some(path) = line.strip_prefix("\t") {
                    work.add_module(path.trim());
                } else if let Some(path) = line.strip_prefix("    ") {
                    work.add_module(path.trim());
                }
            }
        }

        Ok(work)
    }
}
