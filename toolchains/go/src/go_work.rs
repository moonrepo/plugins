// `go.work`

use go_tool::version::from_go_version;
use moon_pdk_api::AnyResult;

#[derive(Debug, Default)]
pub struct GoWork {
    pub modules: Vec<String>,
    pub version: Option<String>,
}

impl GoWork {
    pub fn add_module(&mut self, path: &str) {
        let path = path.trim_matches(['"', '`']);

        if path.starts_with("..") {
            // Ignore since we only support child members
        } else {
            self.modules.push(path.trim_start_matches("./").into());
        }
    }

    // https://go.dev/ref/mod#go-work-file
    pub fn parse(content: impl AsRef<str>) -> AnyResult<Self> {
        let mut work = Self::default();
        let mut block = None;

        for line in content.as_ref().lines() {
            let line = line.split("//").next().unwrap_or_default().trim();

            if line.is_empty() {
                continue;
            }

            // Within a `use (`, `replace (`, or `ignore (` block
            if let Some(directive) = block {
                if line == ")" {
                    block = None;
                } else if directive == "use" {
                    work.add_module(line);
                }

                continue;
            }

            let (directive, value) = match line.find(|c: char| c.is_whitespace() || c == '(') {
                Some(index) => (&line[..index], line[index..].trim()),
                None => (line, ""),
            };

            if value == "(" {
                block = Some(directive);
            } else if directive == "use" {
                work.add_module(value);
            } else if directive == "go" {
                work.version = Some(from_go_version(value));
            }
        }

        Ok(work)
    }
}
