use crate::constants::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Exported {
    pub data: Vec<Dataset>,
}
#[derive(Deserialize)]
pub struct Dataset {
    pub functions: Vec<Function>,
}
#[derive(Deserialize)]
pub struct Function {
    pub name: String,
    pub count: u64,
    regions: Vec<[u64; REGION_FIELDS]>,
    branches: Vec<[u64; BRANCH_FIELDS]>,
}

impl Exported {
    pub fn assert_finite_counts(&self) {
        for function in self.data.iter().flat_map(|data| &data.functions) {
            assert!(
                function.count < SATURATED_COUNT,
                "{}: function",
                function.name
            );
            for region in &function.regions {
                assert!(
                    region[COUNT_INDEX] < SATURATED_COUNT,
                    "{}: region {region:?}",
                    function.name
                );
            }
            for branch in &function.branches {
                assert!(
                    branch[COUNT_INDEX] < SATURATED_COUNT
                        && branch[FALSE_COUNT_INDEX] < SATURATED_COUNT,
                    "{}: branch {branch:?}",
                    function.name
                );
            }
        }
    }
}
