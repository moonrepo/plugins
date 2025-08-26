// // TODO: These tests hang for some reason!

// use starbase_sandbox::{Sandbox, SandboxAssert, create_sandbox};
// use std::time::Duration;

// fn create_moon_sandbox() -> Sandbox {
//     let sandbox = create_sandbox("moon");
//     sandbox.enable_git();
//     sandbox
// }

// fn run_moon<'a>(sandbox: &'a Sandbox, target: &str) -> SandboxAssert<'a> {
//     sandbox.run_bin_with_name("moon", |cmd| {
//         cmd.args(["run", target])
//             .env("MOON_WORKSPACE_ROOT", sandbox.path())
//             .timeout(Duration::from_secs(5));
//     })
// }

// mod node_toolchain_moon {
//     use super::*;

//     #[test]
//     fn standard() {
//         let sandbox = create_moon_sandbox();
//         let assert = run_moon(&sandbox, "tasks:standard");

//         assert!(assert.stderr().contains("stderr"));
//         assert!(assert.stdout().contains("stdout"));
//     }

//     #[test]
//     fn passes_args() {
//         let sandbox = create_moon_sandbox();

//         let assert = sandbox.run_bin_with_name("moon", |cmd| {
//             cmd.args(["run", "tasks:passthroughArgs", "--", "--version"])
//                 .env("MOON_WORKSPACE_ROOT", sandbox.path());
//         });

//         assert.debug();

//         assert!(assert.stdout().contains("Args: --version"));
//     }

//     #[test]
//     fn passes_exec_args() {
//         let sandbox = create_moon_sandbox();
//         let assert = run_moon(&sandbox, "tasks:execArgs");

//         assert.debug();

//         assert!(assert.stdout().contains("--preserve-symlinks"));
//     }

//     #[test]
//     fn handles_cjs() {
//         let sandbox = create_moon_sandbox();

//         let assert = run_moon(&sandbox, "tasks:cjs");

//         assert.debug();

//         assert.success();
//     }

//     #[test]
//     fn handles_mjs() {
//         let sandbox = create_moon_sandbox();

//         let assert = run_moon(&sandbox, "tasks:mjs");

//         assert.debug();

//         assert.success();
//     }

//     #[test]
//     fn can_exec_self() {
//         let sandbox = create_moon_sandbox();
//         let assert = run_moon(&sandbox, "tasks:execBinSelf");
//         assert.debug();

//         assert!(assert.stdout().contains("20.0.0"));
//     }
// }
