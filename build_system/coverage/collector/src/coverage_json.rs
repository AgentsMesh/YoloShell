use serde::Deserialize;

pub const CODE_REGION_KIND: u64 = 0;
pub const EXPANSION_REGION_KIND: u64 = 1;
pub const REGION_FIELD_COUNT: usize = 8;
pub const BRANCH_FIELD_COUNT: usize = 9;
pub const LINE_START_INDEX: usize = 0;
pub const COLUMN_START_INDEX: usize = 1;
pub const LINE_END_INDEX: usize = 2;
pub const COLUMN_END_INDEX: usize = 3;
pub const COUNT_INDEX: usize = 4;
pub const FALSE_COUNT_INDEX: usize = 5;
pub const FILE_ID_INDEX: usize = 5;
pub const BRANCH_FILE_ID_INDEX: usize = 6;
pub const EXPANDED_FILE_ID_INDEX: usize = 6;
pub const BRANCH_EXPANDED_FILE_ID_INDEX: usize = 7;
pub const KIND_INDEX: usize = 7;
pub const BRANCH_KIND_INDEX: usize = 8;

#[derive(Deserialize)]
pub struct Export {
    pub data: Vec<Data>,
}

#[derive(Deserialize)]
pub struct Data {
    pub files: Vec<File>,
    pub functions: Vec<Function>,
}

#[derive(Deserialize)]
pub struct File {
    pub filename: String,
    pub summary: Summary,
}

#[derive(Deserialize)]
pub struct Summary {
    pub branches: SummaryMetric,
    pub functions: SummaryMetric,
    pub lines: SummaryMetric,
    pub regions: SummaryMetric,
}

#[derive(Deserialize)]
pub struct SummaryMetric {
    pub count: u64,
    pub covered: u64,
}

#[derive(Deserialize)]
pub struct Function {
    pub branches: Vec<[u64; BRANCH_FIELD_COUNT]>,
    pub count: u64,
    pub filenames: Vec<String>,
    pub name: String,
    pub regions: Vec<[u64; REGION_FIELD_COUNT]>,
}

pub fn parse(report: &str) -> Result<Data, String> {
    let mut export = serde_json::from_str::<Export>(report)
        .map_err(|error| format!("invalid llvm-cov JSON: {error}"))?;
    if export.data.len() != 1 {
        return Err(format!(
            "llvm-cov JSON contains {} data entries instead of one",
            export.data.len()
        ));
    }
    Ok(export.data.remove(0))
}
