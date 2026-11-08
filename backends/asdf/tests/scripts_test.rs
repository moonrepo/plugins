#[cfg(not(windows))]
mod asdf_backend {
    use proto_pdk_test_utils::*;
    use starbase_sandbox::{Sandbox, create_empty_sandbox};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    // A local asdf plugin repository with the provided bin scripts
    fn create_repo(scripts: &[(&str, &str)]) -> Sandbox {
        let repo = create_empty_sandbox();

        for (name, body) in scripts {
            repo.create_file(
                format!("bin/{name}"),
                format!("#!/usr/bin/env bash\nset -euo pipefail\n{body}\n"),
            );
        }

        // fsmonitor can hang in temporary directories
        for args in [
            &["init"][..],
            &["add", "."],
            &["commit", "--allow-empty", "-m", "init"],
        ] {
            repo.run_git(|cmd| {
                cmd.args(["-c", "core.fsmonitor=false", "-c", "commit.gpgsign=false"])
                    .args(args);
            });
        }

        repo
    }

    async fn create_plugin(
        sandbox: &ProtoWasmSandbox,
        id: &str,
        repo: &Sandbox,
        shortname: Option<&str>,
    ) -> WasmTestWrapper {
        let mut config = HashMap::from([("repository", repo.path().to_str().unwrap())]);

        if let Some(shortname) = shortname {
            config.insert("shortname", shortname);
        }

        sandbox
            .create_plugin_with_config(&format!("asdf:{id}"), |builder| {
                builder.tool_config(&config);
            })
            .await
    }

    fn create_context(dir: &PathBuf) -> PluginContext {
        fs::create_dir_all(dir).unwrap();

        PluginContext {
            tool_dir: VirtualPath::new(dir),
            version: VersionSpec::parse("1.2.3").unwrap(),
            ..Default::default()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_first_version_from_tool_versions() {
        let repo = create_repo(&[]);
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(
            ".tool-versions",
            "nodejs 20.0.0\n  zig   0.13.0  0.12.0 # fallback\n",
        );

        let plugin = create_plugin(&sandbox, "zig", &repo, None).await;
        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                file: ".tool-versions".into(),
                path: VirtualPath::new(sandbox.path().join(".tool-versions")),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version,
            Some(UnresolvedVersionSpec::parse("0.13.0").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn parses_tool_versions_by_shortname() {
        let repo = create_repo(&[]);
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file(".tool-versions", "nodejs 20.1.0\n");

        let plugin = create_plugin(&sandbox, "node", &repo, Some("nodejs")).await;
        let output = plugin
            .parse_version_file(ParseVersionFileInput {
                file: ".tool-versions".into(),
                path: VirtualPath::new(sandbox.path().join(".tool-versions")),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.version,
            Some(UnresolvedVersionSpec::parse("20.1.0").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn passes_empty_query_to_latest_stable() {
        let repo = create_repo(&[("latest-stable", "echo \"1.0.0 2.0.0$1\"")]);
        let sandbox = create_empty_proto_sandbox();

        let plugin = create_plugin(&sandbox, "tool", &repo, None).await;
        let output = plugin
            .resolve_version(ResolveVersionInput {
                initial: UnresolvedVersionSpec::Alias("stable".into()),
                ..Default::default()
            })
            .await;

        assert_eq!(
            output.candidate,
            Some(UnresolvedVersionSpec::parse("2.0.0").unwrap())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn locates_executables_in_all_bin_paths() {
        let repo = create_repo(&[("list-bin-paths", "echo \"bin missing tools/bin\"")]);
        let sandbox = create_empty_proto_sandbox();
        sandbox.create_file("install/bin/foo", "");
        sandbox.create_file("install/tools/bin/bar", "");
        sandbox.create_file("install/tools/bin/lib/baz", "");

        let install_dir = sandbox.path().join("install");
        let plugin = create_plugin(&sandbox, "foo", &repo, None).await;
        let output = plugin
            .locate_executables(LocateExecutablesInput {
                context: create_context(&install_dir),
                install_dir: VirtualPath::new(&install_dir),
            })
            .await;

        assert_eq!(
            output.exes_dirs,
            vec![
                PathBuf::from("bin"),
                PathBuf::from("missing"),
                PathBuf::from("tools/bin")
            ]
        );

        let mut exes = output
            .exes
            .iter()
            .map(|(name, config)| {
                (
                    name.as_str(),
                    config.exe_path.clone().unwrap(),
                    config.primary,
                )
            })
            .collect::<Vec<_>>();
        exes.sort();

        assert_eq!(
            exes,
            vec![
                ("bar", PathBuf::from("tools/bin/bar"), false),
                ("foo", PathBuf::from("bin/foo"), true),
            ]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn captures_multiline_vars_from_exec_env() {
        let repo = create_repo(&[(
            "exec-env",
            "export TOOL_HOME=\"$ASDF_INSTALL_PATH/home\"\nexport TOOL_INIT=\"line1\nkey=line2\"",
        )]);
        let sandbox = create_empty_proto_sandbox();

        let install_dir = sandbox.path().join("install");
        let plugin = create_plugin(&sandbox, "tool", &repo, None).await;
        let output = plugin
            .activate_environment(ActivateEnvironmentInput {
                context: create_context(&install_dir),
                ..Default::default()
            })
            .await;

        let mut env = output.env.into_iter().collect::<Vec<_>>();
        env.sort();

        assert_eq!(
            env,
            vec![
                (
                    "TOOL_HOME".to_owned(),
                    format!("{}/home", install_dir.display())
                ),
                ("TOOL_INIT".to_owned(), "line1\nkey=line2".to_owned()),
            ]
        );
    }
}
