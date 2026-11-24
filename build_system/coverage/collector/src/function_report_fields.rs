use crate::function_report::FunctionMetrics;
use crate::model::Metric;

const ROW_FIELDS: usize = 10;
pub(crate) const REGION_FOUND: usize = 1;
pub(crate) const REGION_MISSED: usize = 2;
pub(crate) const REGION_PERCENT: usize = 3;
pub(crate) const LINE_FOUND: usize = 4;
pub(crate) const LINE_MISSED: usize = 5;
pub(crate) const LINE_PERCENT: usize = 6;
pub(crate) const BRANCH_FOUND: usize = 7;
pub(crate) const BRANCH_MISSED: usize = 8;
pub(crate) const BRANCH_PERCENT: usize = 9;
const PERCENT_SUFFIX: char = '%';

pub fn parse(fields: &[&str], source: &str) -> Result<FunctionMetrics, String> {
    if fields.len() != ROW_FIELDS {
        return Err(format!("malformed function row in {source}: {fields:?}"));
    }
    for index in [REGION_PERCENT, LINE_PERCENT, BRANCH_PERCENT] {
        percent(fields[index], source)?;
    }
    Ok(FunctionMetrics {
        regions: metric(fields[REGION_FOUND], fields[REGION_MISSED], "region")?,
        lines: metric(fields[LINE_FOUND], fields[LINE_MISSED], "line")?,
        branches: metric(fields[BRANCH_FOUND], fields[BRANCH_MISSED], "branch")?,
    })
}

fn metric(found: &str, missed: &str, label: &str) -> Result<Metric, String> {
    let found = found
        .parse::<u64>()
        .map_err(|_| format!("invalid {label} denominator {found:?}"))?;
    let missed = missed
        .parse::<u64>()
        .map_err(|_| format!("invalid missed {label} count {missed:?}"))?;
    let hit = found
        .checked_sub(missed)
        .ok_or_else(|| format!("missed {label} count exceeds denominator"))?;
    Metric { found, hit }.validate(label)
}

fn percent(value: &str, source: &str) -> Result<(), String> {
    let numeric = value
        .strip_suffix(PERCENT_SUFFIX)
        .ok_or_else(|| format!("invalid percentage in {source}"))?;
    let value = numeric
        .parse::<f64>()
        .map_err(|_| format!("invalid percentage in {source}"))?;
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(format!("invalid percentage in {source}"));
    }
    Ok(())
}
