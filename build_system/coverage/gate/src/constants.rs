pub const REQUIRED_PERCENT: u64 = 95;
pub const ORDINAL_MULTIPLICITY_ERROR: &str =
    "synthetic metric ordinal count exceeds merged report multiplicity";
pub const PERCENT_SCALE: u64 = 100;
pub const PERCENT_DECIMAL_PLACES: usize = 2;
pub const FIRST_LINE_NUMBER: usize = 1;
pub const PROGRAM_NAME_ARGUMENTS: usize = 1;
pub const BUILD_WORKSPACE_DIRECTORY_ENV: &str = "BUILD_WORKSPACE_DIRECTORY";

pub const TEST_NAME_TAG: &str = "TN:";

pub const LINES_LABEL: &str = "Lines";
pub const FUNCTIONS_LABEL: &str = "Functions";
pub const REGIONS_LABEL: &str = "Regions";
pub const BRANCHES_LABEL: &str = "Branches";
pub const REPORT_TITLE: &str = "Rust production coverage";
pub const TOOLING_REPORT_TITLE: &str = "Rust build-system production coverage";
pub use yolo_coverage_protocol::{CORE_SCOPE_ARGUMENT, TOOLING_SCOPE_ARGUMENT};
pub const OPTION_PREFIX: &str = "--";
pub const ERROR_PREFIX: &str = "coverage gate error: ";
pub const USAGE: &str = "usage: coverage_gate [--scope=core|--scope=tooling] <bazel LCOV report>";
