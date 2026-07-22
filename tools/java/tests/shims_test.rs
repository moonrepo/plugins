use proto_pdk_test_utils::*;

mod java_tool {
    use super::*;

    #[cfg(not(windows))]
    generate_shims_test!("java-test", ["java", "javac", "jar"]);
}
