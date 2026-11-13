#[cfg(not(windows))]
mod java_tool {
    use proto_pdk_test_utils::*;

    generate_shims_test!("java-test", ["java", "javac", "jar"]);
}
