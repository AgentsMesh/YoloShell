use crate::model::{InstanceMap, Metric, SummaryMap};
use std::collections::BTreeMap;
use yolo_coverage_protocol::MetricKind;

// LLVM JSON saturates wrapped negative counter expressions at this signed limit.
const LLVM_EXECUTION_COUNT_SATURATION: u64 = i64::MAX as u64;

pub fn validate_execution_counts(
    function: &crate::coverage_json::Function,
    source: &str,
) -> Result<(), String> {
    use crate::coverage_json::{COUNT_INDEX, FALSE_COUNT_INDEX};
    let counts = std::iter::once(function.count)
        .chain(function.regions.iter().map(|region| region[COUNT_INDEX]))
        .chain(
            function
                .branches
                .iter()
                .flat_map(|branch| [branch[COUNT_INDEX], branch[FALSE_COUNT_INDEX]]),
        );
    for count in counts {
        if count >= LLVM_EXECUTION_COUNT_SATURATION {
            return Err(format!(
                "invalid LLVM execution count {count} for {} in {source}: saturated or out of signed range",
                function.name
            ));
        }
    }
    Ok(())
}

pub fn validate(summaries: &SummaryMap, instances: &InstanceMap) -> Result<(), String> {
    for source in instances.keys() {
        if !summaries.contains_key(source) {
            return Err(format!(
                "coverage instance has no file summary for {source}"
            ));
        }
    }
    for (source, expected) in summaries {
        let mut groups = BTreeMap::<(u64, u64), BTreeMap<MetricKind, Metric>>::new();
        for instance in instances.get(source).map(Vec::as_slice).unwrap_or_default() {
            let group = groups.entry((instance.line, instance.column)).or_default();
            for kind in MetricKind::ALL {
                let metric = instance.metric(kind)?;
                let current = group.entry(kind).or_default();
                current.found = current.found.max(metric.found);
                current.hit = current.hit.max(metric.hit);
            }
        }
        let mut actual = BTreeMap::<MetricKind, Metric>::new();
        for group in groups.values() {
            for kind in MetricKind::ALL {
                let metric = group
                    .get(&kind)
                    .copied()
                    .ok_or_else(|| format!("incomplete metric group in {source}"))?;
                let total = actual.entry(kind).or_default();
                *total = total.checked_add(metric, kind.label())?;
            }
        }
        for kind in MetricKind::ALL {
            let expected = expected
                .get(&kind)
                .ok_or_else(|| format!("missing {} summary in {source}", kind.label()))?;
            let actual = actual.get(&kind).copied().unwrap_or_default();
            if actual != *expected {
                return Err(format!(
                    "{} summary mismatch in {source}: {}/{} != {}/{}",
                    kind.label(),
                    actual.hit,
                    actual.found,
                    expected.hit,
                    expected.found
                ));
            }
        }
    }
    Ok(())
}
