use crate::inventory::SetInventory;
use crate::lcov_synthetic;
use crate::model::Instance;
use yolo_coverage_protocol::{
    FUNCTION_DATA_TAG, FUNCTION_DEFINITION_TAG, FUNCTIONS_FOUND_TAG, FUNCTIONS_HIT_TAG, RECORD_END,
};

pub fn line(output: &mut String, value: &str) {
    output.push_str(value);
    output.push('\n');
}

pub fn record(output: &mut String, lines: &[String]) {
    for value in lines {
        line(output, value);
    }
    line(output, RECORD_END);
}

pub fn production_record(
    output: &mut String,
    lines: &[String],
    instances: &[Instance],
    source_inventory: &SetInventory,
) -> Result<(), String> {
    let functions = lcov_synthetic::functions(source_inventory, instances)?;
    let mut raw_found = 0_u64;
    let mut raw_hit = 0_u64;
    for value in lines {
        if value.starts_with(FUNCTIONS_FOUND_TAG) || value.starts_with(FUNCTIONS_HIT_TAG) {
            continue;
        }
        if value.starts_with(FUNCTION_DEFINITION_TAG) {
            raw_found = increment(raw_found, "raw function denominator")?;
        }
        if let Some(data) = value.strip_prefix(FUNCTION_DATA_TAG) {
            let count = data
                .split_once(',')
                .ok_or_else(|| format!("malformed raw FNDA {data:?}"))?
                .0
                .parse::<u64>()
                .map_err(|_| format!("invalid raw FNDA count {data:?}"))?;
            if count > 0 {
                raw_hit = increment(raw_hit, "raw covered function count")?;
            }
        }
        line(output, value);
    }
    for function in &functions {
        line(
            output,
            &format!(
                "{FUNCTION_DEFINITION_TAG}{},{}",
                function.line, function.name
            ),
        );
    }
    for function in &functions {
        line(
            output,
            &format!("{FUNCTION_DATA_TAG}{},{}", function.count, function.name),
        );
    }
    let synthetic_found = u64::try_from(functions.len())
        .map_err(|_| "synthetic function denominator overflow".to_owned())?;
    let synthetic_hit = u64::try_from(functions.iter().filter(|item| item.count > 0).count())
        .map_err(|_| "synthetic covered function count overflow".to_owned())?;
    line(
        output,
        &format!("{FUNCTIONS_FOUND_TAG}{}", add(raw_found, synthetic_found)?),
    );
    line(
        output,
        &format!("{FUNCTIONS_HIT_TAG}{}", add(raw_hit, synthetic_hit)?),
    );
    line(output, RECORD_END);
    Ok(())
}

fn increment(value: u64, label: &str) -> Result<u64, String> {
    value
        .checked_add(1)
        .ok_or_else(|| format!("{label} overflow"))
}

fn add(left: u64, right: u64) -> Result<u64, String> {
    left.checked_add(right)
        .ok_or_else(|| "function summary overflow".to_owned())
}
