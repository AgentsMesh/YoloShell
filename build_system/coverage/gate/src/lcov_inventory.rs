use crate::lcov::ParsedReport;
use std::collections::BTreeMap;
use yolo_coverage_protocol::MAXIMUM_MERGED_REPORT_COUNT;

pub use crate::instance_inventory::InstanceInventories;
pub use crate::source_inventory::SourceInventories;

pub fn validate(report: &ParsedReport) -> Result<(), String> {
    crate::source_inventory::validate(report)?;
    crate::instance_inventory::validate(report)
}

pub fn presence(count: u64, label: &str) -> Result<(), String> {
    if !(1..=MAXIMUM_MERGED_REPORT_COUNT).contains(&count) {
        return Err(format!(
            "{label} multiplicity is outside the protocol limit"
        ));
    }
    Ok(())
}

pub fn marker_totals(
    report: &ParsedReport,
    totals: &BTreeMap<String, u64>,
    label: &str,
) -> Result<(), String> {
    for (source, marker) in &report.markers {
        if totals.get(source) != Some(marker) {
            return Err(format!(
                "{label} multiplicity differs from marker in {source}"
            ));
        }
    }
    Ok(())
}

pub fn add_source(
    totals: &mut BTreeMap<String, u64>,
    source: &str,
    value: u64,
    label: &str,
) -> Result<(), String> {
    let total = totals.entry(source.to_owned()).or_default();
    *total = total
        .checked_add(value)
        .ok_or_else(|| format!("{label} overflow"))?;
    Ok(())
}
