use crate::constants::{FIRST_LINE_NUMBER, TEST_NAME_TAG};
use crate::lcov_totals;
use crate::metrics::Totals;
use std::collections::{BTreeMap, BTreeSet};
use yolo_coverage_protocol::{MetricKind, ProductionScope, RECORD_END, SOURCE_TAG};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LogicalKey {
    pub column: u64,
    pub identity: String,
    pub line: u64,
    pub source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Header {
    pub count: u64,
    pub found: u64,
    pub vector_len: u64,
}

#[derive(Debug, Default)]
pub struct InstanceRecords {
    pub headers: BTreeMap<MetricKind, Header>,
    pub ordinals: BTreeMap<MetricKind, BTreeMap<u64, u64>>,
    pub signature: String,
}

#[derive(Debug, Default)]
pub struct ParsedReport {
    pub instance_inventories: crate::lcov_inventory::InstanceInventories,
    pub instances: BTreeMap<LogicalKey, InstanceRecords>,
    pub markers: BTreeMap<String, u64>,
    pub production: BTreeSet<String>,
    pub source_inventories: crate::lcov_inventory::SourceInventories,
}

pub(super) struct Record<'a> {
    pub(super) lines: Vec<&'a str>,
    pub(super) source: &'a str,
}

pub fn parse(report: &str) -> Result<Totals, String> {
    parse_scope(report, ProductionScope::Core)
}

pub fn parse_scope(report: &str, scope: ProductionScope) -> Result<Totals, String> {
    let mut parsed = ParsedReport::default();
    let mut current: Option<Record<'_>> = None;
    for (index, raw_line) in report.lines().enumerate() {
        let line_number = index + FIRST_LINE_NUMBER;
        let line = raw_line.trim_end_matches('\r');
        if let Some(source) = line.strip_prefix(SOURCE_TAG) {
            if source.is_empty() || current.is_some() {
                return Err(format!("invalid nested LCOV source at line {line_number}"));
            }
            current = Some(Record {
                lines: Vec::new(),
                source,
            });
        } else if line == RECORD_END {
            let record = current
                .take()
                .ok_or_else(|| format!("record end without source at line {line_number}"))?;
            crate::lcov_finish::finish(record, &mut parsed)?;
        } else if let Some(record) = current.as_mut() {
            record.lines.push(line);
        } else if !line.is_empty() && !line.starts_with(TEST_NAME_TAG) {
            return Err(format!(
                "LCOV content outside a record at line {line_number}"
            ));
        }
    }
    if let Some(record) = current {
        return Err(format!("unterminated LCOV source {:?}", record.source));
    }
    lcov_totals::calculate(parsed, scope)
}
