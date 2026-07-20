#[cfg(not(windows))]
mod node_depman_tool {
    use proto_pdk_test_utils::*;

    mod npm {
        use super::*;

        generate_shims_test!("npm-test", ["npm", "npx", "node-gyp"]);
    }

    mod pnpm {
        use super::*;

        // The pn and pnx shims are created for pnpm >= 11 (the latest)
        generate_shims_test!("pnpm-test", ["pnpm", "pnpx", "pn", "pnx"]);
    }

    mod yarn {
        use super::*;

        generate_shims_test!("yarn-test", ["yarn", "yarnpkg"]);
    }
}
