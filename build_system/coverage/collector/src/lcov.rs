use crate::inventory;
use crate::lcov_write;
use crate::model::InstanceMap;
use crate::normalize;
use std::collections::BTreeSet;
use yolo_coverage_protocol::{RECORD_END, SOURCE_TAG, production_key};

struct Record {
    lines: Vec<String>,
    source: String,
}

pub fn augment(
    report: &str,
    instances: &InstanceMap,
    expected_sources: &BTreeSet<String>,
) -> Result<String, String> {
    let source_inventory = inventory::sources(expected_sources)?;
    let mut output = String::new();
    let mut current: Option<Record> = None;
    let mut sources = BTreeSet::new();
    for line in report.lines() {
        if let Some(source) = line.strip_prefix(SOURCE_TAG) {
            if source.is_empty() || current.is_some() {
                return Err(format!("invalid nested LCOV source {source:?}"));
            }
            current = Some(Record {
                lines: vec![line.to_owned()],
                source: normalize::lcov_source(source).to_owned(),
            });
        } else if line == RECORD_END {
            let record = current
                .take()
                .ok_or_else(|| "LCOV record end without source".to_owned())?;
            finish(
                record,
                instances,
                &source_inventory,
                &mut sources,
                &mut output,
            )?;
        } else if let Some(record) = current.as_mut() {
            record.lines.push(line.to_owned());
        } else {
            lcov_write::line(&mut output, line);
        }
    }
    if let Some(record) = current {
        return Err(format!("unterminated LCOV source {}", record.source));
    }
    if &sources != expected_sources {
        return Err(format!(
            "LCOV production sources differ from JSON summaries: {sources:?} != {expected_sources:?}"
        ));
    }
    Ok(output)
}

fn finish(
    record: Record,
    instances: &InstanceMap,
    source_inventory: &inventory::SetInventory,
    sources: &mut BTreeSet<String>,
    output: &mut String,
) -> Result<(), String> {
    let Some(source) = production_key(&record.source)? else {
        lcov_write::record(output, &record.lines);
        return Ok(());
    };
    if !sources.insert(source.clone()) {
        return Err(format!("duplicate production LCOV source {source}"));
    }
    lcov_write::production_record(
        output,
        &record.lines,
        instances
            .get(&source)
            .map(Vec::as_slice)
            .unwrap_or_default(),
        source_inventory,
    )
}
