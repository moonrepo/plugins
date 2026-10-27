use proto_pdk_test_utils::*;
use starbase_sandbox::locate_fixture;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

fn bind_json() -> (String, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();

    (format!("http://{address}/versions.json"), listener)
}

fn serve_json(listener: TcpListener, body: &'static str) -> thread::JoinHandle<bool> {
    listener.set_nonblocking(true).unwrap();

    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);

        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut request = [0; 1024];
                    assert!(stream.read(&mut request).unwrap() > 0);
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();

                    return true;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return false;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("Failed to accept index request: {error}"),
            }
        }
    })
}

mod schema_tool {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_nested_index_versions_in_both_schema_formats() {
        for format in [1, 2] {
            let (endpoint, listener) = bind_json();
            let sandbox = create_empty_proto_sandbox();
            let schema = if format == 1 {
                format!(
                    "name = \"schema-test\"\ntype = \"cli\"\n\n[resolve]\nmanifest-url = \"{endpoint}\"\nmanifest-array-key = \"releases\"\nmanifest-version-key = \"tag\"\n"
                )
            } else {
                format!(
                    "format = \"2\"\n\n[metadata]\nname = \"schema-test\"\ntype = \"command-line\"\n\n[resolve]\nindex_url = \"{endpoint}\"\nindex_array_key = \"releases\"\nindex_version_key = \"tag\"\n"
                )
            };
            sandbox.create_file("index.toml", &schema);
            let plugin = sandbox
                .create_schema_plugin("schema-test", sandbox.path().join("index.toml"))
                .await;

            let server = serve_json(
                listener,
                r#"{"releases":[{"tag":"1.2.3"},{"tag":"2.3.4"}]}"#,
            );
            let output = plugin.load_versions(LoadVersionsInput::default()).await;
            assert!(
                server.join().unwrap(),
                "format {format} did not request the index"
            );

            assert!(
                output
                    .versions
                    .contains(&VersionSpec::parse("1.2.3").unwrap()),
                "format {format} failed to load 1.2.3"
            );
            assert!(
                output
                    .versions
                    .contains(&VersionSpec::parse("2.3.4").unwrap()),
                "format {format} failed to load 2.3.4"
            );
        }
    }

    generate_resolve_versions_tests!(
        "schema-test",
        {
            "1.0.3" => "1.0.3",
            "1.4" => "1.4.0",
            "1.5" => "1.5.1",
        },
        Some(locate_fixture("schemas").join("base.toml"))
    );

    #[tokio::test(flavor = "multi_thread")]
    async fn loads_versions_from_git_tags() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin("schema-test", locate_fixture("schemas").join("base.toml"))
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sets_latest_alias() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin("schema-test", locate_fixture("schemas").join("base.toml"))
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(output.latest.is_some());
        assert!(output.aliases.contains_key("latest"));
        assert_eq!(output.aliases.get("latest"), output.latest.as_ref());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn version_pattern_supports_common_classes() {
        let sandbox = create_empty_proto_sandbox();
        let plugin = sandbox
            .create_schema_plugin(
                "schema-test",
                locate_fixture("schemas").join("version-pattern.toml"),
            )
            .await;

        let output = plugin.load_versions(LoadVersionsInput::default()).await;

        assert!(!output.versions.is_empty());
    }
}
