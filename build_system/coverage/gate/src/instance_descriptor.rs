use crate::lcov::{InstanceRecords, ParsedReport};
use std::collections::BTreeMap;
use yolo_coverage_protocol::{MetricKind, MetricShape, instance_digest};

pub type ActualInstances<'a> = BTreeMap<String, BTreeMap<String, &'a InstanceRecords>>;

pub fn collect(report: &ParsedReport) -> Result<ActualInstances<'_>, String> {
    let mut actual = report
        .production
        .iter()
        .map(|source| (source.clone(), BTreeMap::new()))
        .collect::<BTreeMap<_, _>>();
    for (key, records) in &report.instances {
        let digest = instance_digest(
            key.line,
            key.column,
            &key.identity,
            &records.signature,
            &shapes(records)?,
        );
        let source = actual
            .get_mut(&key.source)
            .ok_or_else(|| "coverage instance appears outside production sources".to_owned())?;
        if source.insert(digest, records).is_some() {
            return Err("coverage instances have a digest collision".to_owned());
        }
    }
    Ok(actual)
}

pub fn compare(
    actual: &ActualInstances<'_>,
    expected: &BTreeMap<(String, String), u64>,
) -> Result<(), String> {
    for (source, instances) in actual {
        let declared = expected
            .iter()
            .filter(|((candidate, _), _)| candidate == source)
            .map(|((_, instance), count)| (instance, count))
            .collect::<BTreeMap<_, _>>();
        if instances.keys().ne(declared.keys().copied()) {
            return Err(format!(
                "instance inventory differs from metric headers in {source}"
            ));
        }
        for (instance, records) in instances {
            let count = declared[instance];
            if records
                .headers
                .values()
                .any(|header| header.count != *count)
            {
                return Err("metric header multiplicity differs from instance inventory".to_owned());
            }
        }
    }
    Ok(())
}

fn shapes(records: &InstanceRecords) -> Result<[MetricShape; 4], String> {
    if records.headers.len() != MetricKind::ALL.len() {
        return Err("synthetic instance does not contain every metric header".to_owned());
    }
    let shape = |kind| {
        records
            .headers
            .get(&kind)
            .map(|header| MetricShape {
                found: header.found,
                vector_len: header.vector_len,
            })
            .ok_or_else(|| format!("synthetic instance has no {} header", kind.label()))
    };
    Ok([
        shape(MetricKind::Lines)?,
        shape(MetricKind::Functions)?,
        shape(MetricKind::Regions)?,
        shape(MetricKind::Branches)?,
    ])
}
