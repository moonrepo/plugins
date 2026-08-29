pub mod v1;
pub mod v2;

pub const VERSION_REGEX: &str = r"^v?((?<major>[0-9]+)\.(?<minor>[0-9]+)\.(?<patch>[0-9]+)(?<pre>-[0-9a-zA-Z\.]+)?(?<build>\+[-0-9a-zA-Z\.]+)?)$";

pub enum Schema {
    V1(v1::SchemaV1),
    V2(v2::SchemaV2),
}
