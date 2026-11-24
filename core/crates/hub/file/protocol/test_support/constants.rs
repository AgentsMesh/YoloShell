use yolo_crypto::Sha256Digest;

pub const RELATIVE: &str = "src/main.rs";
pub const OTHER_RELATIVE: &str = "src/lib.rs";
pub const ROOT_NAME: &str = "Project";
pub const ROOT_PATH: &str = "/srv/project";
pub const PAYLOAD: &[u8] = b"hello";
pub const SAMPLE_COMPONENT: &str = "a";
pub const VERSION: Sha256Digest = Sha256Digest::repeated(1);
pub const OTHER_VERSION: Sha256Digest = Sha256Digest::repeated(2);
pub const SCHEMA_FIELD: &str = "schema_version";
pub const EXTRA_FIELD: &str = "unexpected";
pub const VERSION_FIELD: &str = "expected_version";
pub const PATH_FIELD: &str = "path";
pub const NAME_FIELD: &str = "name";
