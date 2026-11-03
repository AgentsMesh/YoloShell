use crate::coverage_json::{CODE_REGION_KIND, Data, File, Function, Summary, SummaryMetric};
use crate::function_report::{FunctionMetrics, FunctionReport};
use crate::model::{Metric, SummaryMap};
use std::path::Path;

pub use crate::test_constants::{BAZEL_WITNESS_PREFIX, FUNCTION, OTHER_FUNCTION, TEST_WORKSPACE};

pub const ROOT: &str = "/workspace";
pub const SOURCE: &str = "build_system/fixture/src/lib.rs";
pub const OTHER_SOURCE: &str = "build_system/fixture/src/other.rs";
pub const TEST_SOURCE: &str = "build_system/fixture/unit_tests/case.rs";
pub const POSITION: u64 = 1;
pub const END_COLUMN: u64 = 5;
pub const COUNT: u64 = 3;
pub const FILE_INDEX: u64 = 0;
pub const SINGLE_METRIC: u64 = 1;
pub const BRANCH_OUTCOMES: u64 = 2;
pub const OUTSIDE_FILE_INDEX: u64 = 99;

pub fn function() -> Function {
    Function {
        count: COUNT,
        name: FUNCTION.to_owned(),
        filenames: vec![SOURCE.to_owned()],
        regions: vec![[
            POSITION,
            POSITION,
            POSITION,
            END_COLUMN,
            COUNT,
            FILE_INDEX,
            FILE_INDEX,
            CODE_REGION_KIND,
        ]],
        branches: vec![[
            POSITION,
            POSITION,
            POSITION,
            END_COLUMN,
            COUNT,
            COUNT,
            FILE_INDEX,
            FILE_INDEX,
            CODE_REGION_KIND,
        ]],
    }
}

pub fn report() -> FunctionReport {
    let covered = Metric {
        found: SINGLE_METRIC,
        hit: SINGLE_METRIC,
    };
    FunctionReport::from([(
        (SOURCE.to_owned(), FUNCTION.to_owned()),
        FunctionMetrics {
            lines: covered,
            regions: covered,
            branches: Metric {
                found: BRANCH_OUTCOMES,
                hit: BRANCH_OUTCOMES,
            },
        },
    )])
}

pub fn data(functions: Vec<Function>) -> Data {
    let metric = || SummaryMetric {
        count: SINGLE_METRIC,
        covered: SINGLE_METRIC,
    };
    Data {
        files: vec![File {
            filename: SOURCE.to_owned(),
            summary: Summary {
                lines: metric(),
                functions: metric(),
                regions: metric(),
                branches: SummaryMetric {
                    count: BRANCH_OUTCOMES,
                    covered: BRANCH_OUTCOMES,
                },
            },
        }],
        functions,
    }
}

pub fn summaries() -> SummaryMap {
    crate::coverage_data::summaries(&data(vec![function()]), Path::new(ROOT)).unwrap()
}
