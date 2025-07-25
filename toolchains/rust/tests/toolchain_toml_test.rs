use moon_pdk_api::VirtualPath;
use rust_toolchain::toolchain_toml::*;
use starbase_utils::toml::TomlValue;
use std::path::PathBuf;

mod toolchain_toml {
    use super::*;

    mod set_channel {
        use super::*;

        #[test]
        fn adds_if_not_set() {
            let mut tc = ToolchainToml {
                path: VirtualPath::Real(PathBuf::from("/base/rust-toolchain.toml")),
                ..Default::default()
            };

            assert_eq!(tc.data.toolchain.channel, None);

            assert!(tc.set_channel("stable").unwrap());

            assert_eq!(tc.data.toolchain.channel.unwrap(), "stable");
            assert_eq!(tc.dirty, ["toolchain.channel"]);
        }

        #[test]
        fn doesnt_add_if_empty() {
            let mut tc = ToolchainToml {
                path: VirtualPath::Real(PathBuf::from("/base/rust-toolchain.toml")),
                ..Default::default()
            };

            assert_eq!(tc.data.toolchain.channel, None);

            assert!(!tc.set_channel("").unwrap());

            assert_eq!(tc.data.toolchain.channel, None);
            assert!(tc.dirty.is_empty());
        }

        #[test]
        fn doesnt_add_if_same_value() {
            let mut tc = ToolchainToml {
                path: VirtualPath::Real(PathBuf::from("/base/rust-toolchain.toml")),
                data: BaseToolchainToml {
                    toolchain: ToolchainSection {
                        channel: Some("stable".into()),
                    },
                },
                ..Default::default()
            };

            assert_eq!(tc.data.toolchain.channel.as_ref().unwrap(), "stable");

            assert!(!tc.set_channel("stable").unwrap());

            assert_eq!(tc.data.toolchain.channel.unwrap(), "stable");
            assert!(tc.dirty.is_empty());
        }

        #[test]
        fn saves_field_if_set() {
            let mut tc = ToolchainToml {
                path: VirtualPath::Real(PathBuf::from("/base/rust-toolchain.toml")),
                ..Default::default()
            };

            tc.set_channel("stable").unwrap();

            let mut root = TomlValue::Table(Default::default());

            tc.save_field("toolchain.channel", &mut root).unwrap();

            assert_eq!(
                root["toolchain"]["channel"],
                TomlValue::String("stable".into())
            );
        }

        #[test]
        fn doesnt_save_field_if_not_set() {
            let tc = ToolchainToml {
                path: VirtualPath::Real(PathBuf::from("/base/rust-toolchain.toml")),
                ..Default::default()
            };

            let mut root = TomlValue::Table(Default::default());

            tc.save_field("toolchain.channel", &mut root).unwrap();

            assert!(root.as_table().unwrap().is_empty());
        }
    }
}
