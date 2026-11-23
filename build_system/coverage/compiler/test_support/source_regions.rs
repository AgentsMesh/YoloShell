use crate::constants::*;
use serde_json::Value;
use std::path::Path;

type Position = (u64, u64);

#[derive(Clone, Copy)]
pub struct Mapping<'a> {
    pub function: &'a Value,
    pub region: &'a Value,
    start: Position,
    end: Position,
}

pub fn fixture_file(function: &Value, region: &Value, file_index: usize) -> bool {
    let file_id = usize::try_from(region[file_index].as_u64().unwrap()).unwrap();
    let filename = function[FILENAMES].as_array().unwrap()[file_id]
        .as_str()
        .unwrap();
    Path::new(filename).ends_with(SOURCE)
}

pub fn code<'a>(functions: &[&'a Value], marker: &str, variant: &str) -> Mapping<'a> {
    closest(
        functions,
        marker,
        variant,
        REGIONS,
        REGION_FILE_ID_INDEX,
        REGION_KIND_INDEX,
        CODE_REGION_KIND,
    )
}

pub fn branch<'a>(functions: &[&'a Value], marker: &str, variant: &str) -> Mapping<'a> {
    closest(
        functions,
        marker,
        variant,
        BRANCHES,
        BRANCH_FILE_ID_INDEX,
        BRANCH_KIND_INDEX,
        BRANCH_REGION_KIND,
    )
}

fn closest<'a>(
    functions: &[&'a Value],
    marker: &str,
    variant: &str,
    field: &str,
    file_index: usize,
    kind_index: usize,
    kind: u64,
) -> Mapping<'a> {
    let point = source_position(marker);
    let mut matches = Vec::new();
    for &function in functions {
        for region in function[field].as_array().unwrap() {
            if region[kind_index].as_u64() != Some(kind)
                || !fixture_file(function, region, file_index)
            {
                continue;
            }
            let start = position(region, START_LINE_INDEX, START_COLUMN_INDEX);
            let end = position(region, END_LINE_INDEX, END_COLUMN_INDEX);
            assert!(
                start <= end,
                "{variant}: reversed {field} mapping: {region}"
            );
            if start <= point && point < end {
                matches.push(Mapping {
                    function,
                    region,
                    start,
                    end,
                });
            }
        }
    }
    // Select by source geometry only. An enclosing region cannot substitute for a more
    // specific one, and equal or overlapping incomparable minima are ambiguous evidence.
    let specific = matches
        .iter()
        .filter(|candidate| {
            !matches.iter().any(|other| {
                (candidate.start, candidate.end) != (other.start, other.end)
                    && candidate.start <= other.start
                    && other.end <= candidate.end
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        specific.len(),
        SINGLE,
        "{variant}: missing or ambiguous {field} mapping for {marker:?} at {point:?}; candidates: {:?}",
        matches
            .iter()
            .map(|mapping| (&mapping.function[NAME], mapping.region))
            .collect::<Vec<_>>()
    );
    *specific[0]
}

fn source_position(marker: &str) -> Position {
    let occurrences = SOURCE_TEXT.match_indices(marker).collect::<Vec<_>>();
    assert_eq!(
        occurrences.len(),
        SINGLE,
        "source marker must be unique: {marker:?}"
    );
    let prefix = &SOURCE_TEXT[..occurrences[0].0];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + SINGLE;
    let column = prefix.rsplit('\n').next().unwrap().len() + SINGLE;
    (u64::try_from(line).unwrap(), u64::try_from(column).unwrap())
}

fn position(region: &Value, line: usize, column: usize) -> Position {
    (
        region[line].as_u64().unwrap(),
        region[column].as_u64().unwrap(),
    )
}
