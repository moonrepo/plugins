use rustc_hash::FxHashMap;
use serde::Deserialize;

/// Output of `npm view <id> --json dist-tags`.
// npm v12+ wraps object results in an array
#[derive(Deserialize)]
#[serde(untagged)]
pub enum DistTags {
    Map(FxHashMap<String, String>),
    List(Vec<FxHashMap<String, String>>),
}

impl DistTags {
    pub fn into_map(self) -> FxHashMap<String, String> {
        match self {
            Self::Map(tags) => tags,
            Self::List(list) => list.into_iter().next().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_utils::json;

    fn parse(data: &str) -> FxHashMap<String, String> {
        json::parse::<DistTags>(data).unwrap().into_map()
    }

    #[test]
    fn parses_npm11_map() {
        let tags = parse(r#"{ "next": "17.1.4-0", "latest": "23.1.0" }"#);

        assert_eq!(tags.len(), 2);
        assert_eq!(tags["latest"], "23.1.0");
    }

    #[test]
    fn parses_npm12_list() {
        let tags = parse(r#"[{ "next": "17.1.4-0", "latest": "23.1.0" }]"#);

        assert_eq!(tags.len(), 2);
        assert_eq!(tags["latest"], "23.1.0");
    }

    #[test]
    fn parses_empty_list() {
        assert!(parse("[]").is_empty());
    }
}
