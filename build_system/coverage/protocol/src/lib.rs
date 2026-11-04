mod protocol_digest;
mod protocol_parse;
mod protocol_source;
mod protocol_tokens;
mod scope_constants;

pub use scope_constants::{CORE_SCOPE_ARGUMENT, TOOLING_SCOPE_ARGUMENT};

pub use protocol_digest::{
    digest_bytes, encode_fields, instance_digest, instance_inventory_digest,
    source_inventory_digest,
};
pub use protocol_parse::parse_token;
pub use protocol_source::{ProductionScope, production_key, production_scope};
pub use protocol_tokens::{
    header_token, instance_inventory_token, instance_member_token, ordinal_token,
    source_inventory_token,
};

pub const SOURCE_TAG: &str = "SF:";
pub const FUNCTION_DEFINITION_TAG: &str = "FN:";
pub const FUNCTION_DATA_TAG: &str = "FNDA:";
pub const FUNCTIONS_FOUND_TAG: &str = "FNF:";
pub const FUNCTIONS_HIT_TAG: &str = "FNH:";
pub const RECORD_END: &str = "end_of_record";
pub const TOKEN_ROOT: &str = "__yolo_metric__/";
pub const MARKER_TOKEN: &str = "__yolo_metric__/marker";
pub const SOURCE_INVENTORY_PREFIX: &str = "__yolo_metric__/sources/";
pub const INSTANCE_INVENTORY_PREFIX: &str = "__yolo_metric__/instances/";
pub const INSTANCE_MEMBER_PREFIX: &str = "__yolo_metric__/member/";
pub const HEADER_PREFIX: &str = "__yolo_metric__/header/";
pub const ORDINAL_PREFIX: &str = "__yolo_metric__/ordinal/";
pub const RUST_SOURCE_ROOT: &str = "core/src/";
pub const RUST_COMPONENT_SOURCE_ROOT: &str = "core/crates/";
pub const CLIENT_SOURCE_ROOT: &str = "clients/";
pub const BUILD_SYSTEM_SOURCE_ROOT: &str = "build_system/";
pub const MINIMUM_SOURCE_POSITION: u64 = 1;
pub const MAXIMUM_SOURCE_POSITION: u64 = u32::MAX as u64;

// Bounds coverage-map expansion before it can allocate attacker-sized vectors.
pub const MAXIMUM_INSTANCE_VECTOR_LENGTH: u64 = 1_000_000;
pub const MAXIMUM_MERGED_REPORT_COUNT: u64 = 1_000_000;

pub const SHA256_BYTES: usize = 32;
pub const HEX_CHARACTERS_PER_BYTE: usize = 2;
pub const SHA256_HEX_LENGTH: usize = SHA256_BYTES * HEX_CHARACTERS_PER_BYTE;

pub(crate) const SOURCE_INVENTORY_DOMAIN: &str = "yolo.coverage.sources.v1";
pub(crate) const INSTANCE_INVENTORY_DOMAIN: &str = "yolo.coverage.instances.v1";
pub(crate) const INSTANCE_IDENTITY_DOMAIN: &str = "yolo.coverage.instance.v1";
pub(crate) const CURRENT_DIRECTORY_PREFIX: &str = "./";
pub(crate) const CURRENT_DIRECTORY_COMPONENT: &str = ".";
pub(crate) const PARENT_DIRECTORY_COMPONENT: &str = "..";
pub(crate) const RUST_SOURCE_EXTENSION: &str = ".rs";
pub(crate) const TEST_DIRECTORY: &str = "test";
pub(crate) const TESTS_DIRECTORY: &str = "tests";
pub(crate) const TEST_SUPPORT_DIRECTORY: &str = "test_support";
pub(crate) const TEST_SUPPORT_PACKAGE_SUFFIX: &str = "_test_support";
pub(crate) const FIXTURE_DIRECTORY: &str = "fixtures";
pub(crate) const GENERATED_DIRECTORY: &str = "generated";
pub(crate) const SOURCE_DIRECTORY: &str = "src";
pub(crate) const TESTS_SUFFIX: &str = "_tests";
pub(crate) const TEST_SOURCE: &str = "test.rs";
pub(crate) const TESTS_SOURCE: &str = "tests.rs";
pub(crate) const TEST_SOURCE_PREFIX: &str = "test_";
pub(crate) const TESTS_SOURCE_PREFIX: &str = "tests_";
pub(crate) const TESTS_SOURCE_SUFFIX: &str = "_tests.rs";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MetricKind {
    Lines,
    Functions,
    Regions,
    Branches,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MetricShape {
    pub found: u64,
    pub vector_len: u64,
}

impl MetricKind {
    pub const ALL: [Self; 4] = [Self::Lines, Self::Functions, Self::Regions, Self::Branches];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Lines => "Lines",
            Self::Functions => "Functions",
            Self::Regions => "Regions",
            Self::Branches => "Branches",
        }
    }

    pub const fn path(self) -> &'static str {
        match self {
            Self::Lines => "lines",
            Self::Functions => "functions",
            Self::Regions => "regions",
            Self::Branches => "branches",
        }
    }

    pub fn from_path(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.path() == value)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum FunctionToken<'a> {
    Marker,
    SourceInventory {
        inventory: &'a str,
        members: u64,
    },
    InstanceInventory {
        inventory: &'a str,
        members: u64,
    },
    InstanceMember {
        inventory: &'a str,
        instance: &'a str,
    },
    Header {
        kind: MetricKind,
        line: u64,
        column: u64,
        identity: &'a str,
        signature: &'a str,
        vector_len: u64,
        found: u64,
    },
    Ordinal {
        kind: MetricKind,
        line: u64,
        column: u64,
        identity: &'a str,
        signature: &'a str,
        ordinal: u64,
    },
}
