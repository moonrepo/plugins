// `go.work`

use crate::parsers;
use go_tool::version::from_go_version;
use moon_pdk_api::AnyResult;

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

        for directive in parsers::go_work::parse(content.as_ref()) {
            match (directive.name.as_str(), directive.args.first()) {
                ("go", Some(version)) => work.version = Some(from_go_version(version)),
                ("use", Some(path)) => work.add_module(path),
                _ => {}
            }
        }

        Ok(work)
    }
}
