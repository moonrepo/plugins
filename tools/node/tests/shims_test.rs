#[cfg(not(windows))]
use proto_pdk_test_utils::*;

#[cfg(not(windows))]
mod node_tool {
    use super::*;

    generate_shims_test!("node-test", ["node"]);
}
