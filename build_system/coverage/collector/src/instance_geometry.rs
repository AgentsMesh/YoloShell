use crate::coverage_json::*;
use crate::normalize;
use crate::segments::{Region, RegionKind};
use std::path::Path;
use yolo_coverage_protocol::production_key;

const MINIMUM_COORDINATE: u64 = 1;
const MAXIMUM_COORDINATE: u64 = u32::MAX as u64;

pub struct Geometry {
    pub column: u64,
    pub filenames: Vec<String>,
    pub line: u64,
    pub main_regions: Vec<Region>,
    pub source: String,
}

pub fn parse(function: &Function, normalization_root: &Path) -> Result<Option<Geometry>, String> {
    let filenames = filenames(function, normalization_root)?;
    let main_id = main_file_id(function, filenames.len())?;
    let source = filenames[main_id].clone();
    let Some(source) = production_key(&source)? else {
        return Ok(None);
    };
    validate_map(function, filenames.len())?;
    crate::validation::validate_execution_counts(function, &source)?;
    let mut main_regions = Vec::new();
    let mut start = None;
    for value in &function.regions {
        if usize::try_from(value[FILE_ID_INDEX]) == Ok(main_id) {
            start.get_or_insert((value[LINE_START_INDEX], value[COLUMN_START_INDEX]));
            main_regions.push(region(value)?);
        }
    }
    let (line, column) =
        start.ok_or_else(|| format!("function {} has no main-file region", function.name))?;
    Ok(Some(Geometry {
        column,
        filenames,
        line,
        main_regions,
        source,
    }))
}

fn filenames(function: &Function, root: &Path) -> Result<Vec<String>, String> {
    if function.name.is_empty() || function.filenames.is_empty() {
        return Err("coverage function has an empty name or filename set".to_owned());
    }
    function
        .filenames
        .iter()
        .map(|filename| {
            let normalized = normalize::source(filename, root);
            (!normalized.is_empty())
                .then_some(normalized)
                .ok_or_else(|| format!("empty filename in function {}", function.name))
        })
        .collect()
}

fn main_file_id(function: &Function, filename_count: usize) -> Result<usize, String> {
    let mut main = vec![true; filename_count];
    for region in &function.regions {
        if region[KIND_INDEX] == EXPANSION_REGION_KIND {
            let expanded = index(
                region[EXPANDED_FILE_ID_INDEX],
                filename_count,
                "expanded file",
            )?;
            main[expanded] = false;
        }
    }
    main.iter()
        .position(|candidate| *candidate)
        .ok_or_else(|| format!("function {} has no main file", function.name))
}

fn validate_map(function: &Function, count: usize) -> Result<(), String> {
    for region in &function.regions {
        index(region[FILE_ID_INDEX], count, "region file")?;
        if region[KIND_INDEX] == EXPANSION_REGION_KIND {
            index(region[EXPANDED_FILE_ID_INDEX], count, "expanded file")?;
        }
        self::region(region)?;
    }
    for branch in &function.branches {
        index(branch[BRANCH_FILE_ID_INDEX], count, "branch file")?;
        index(
            branch[BRANCH_EXPANDED_FILE_ID_INDEX],
            count,
            "branch expanded file",
        )?;
        coordinates(branch)?;
    }
    Ok(())
}

fn region(value: &[u64; REGION_FIELD_COUNT]) -> Result<Region, String> {
    coordinates(value)?;
    Ok(Region {
        start_line: value[LINE_START_INDEX],
        start_column: value[COLUMN_START_INDEX],
        end_line: value[LINE_END_INDEX],
        end_column: value[COLUMN_END_INDEX],
        count: value[COUNT_INDEX],
        kind: RegionKind::from_u64(value[KIND_INDEX]).map_err(segment_error)?,
    })
}

fn coordinates<const N: usize>(value: &[u64; N]) -> Result<(), String> {
    let start = (value[LINE_START_INDEX], value[COLUMN_START_INDEX]);
    let end = (value[LINE_END_INDEX], value[COLUMN_END_INDEX]);
    if [start.0, start.1, end.0, end.1]
        .iter()
        .any(|coordinate| !(MINIMUM_COORDINATE..=MAXIMUM_COORDINATE).contains(coordinate))
        || start > end
    {
        return Err("invalid LLVM coverage map coordinates".to_owned());
    }
    Ok(())
}

fn index(value: u64, count: usize, label: &str) -> Result<usize, String> {
    let value = usize::try_from(value).map_err(|_| format!("invalid {label} ID"))?;
    (value < count)
        .then_some(value)
        .ok_or_else(|| format!("invalid {label} ID"))
}

fn segment_error(error: impl std::fmt::Display) -> String {
    format!("invalid LLVM coverage map: {error}")
}
