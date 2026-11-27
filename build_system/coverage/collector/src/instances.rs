use crate::coverage_json::{CODE_REGION_KIND, COUNT_INDEX, Data};
use crate::function_report::{FunctionMetrics, FunctionReport};
use crate::instance_geometry;
use crate::model::{Instance, InstanceMap, Metric};
use crate::segments::{build_segments, line_coverage};
use crate::signature;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use yolo_coverage_protocol::MetricKind;

pub fn parse(
    data: &Data,
    mut report: FunctionReport,
    normalization_root: &Path,
) -> Result<InstanceMap, String> {
    let mut instances = InstanceMap::new();
    let mut identities = BTreeSet::new();
    for function in &data.functions {
        let Some(geometry) = instance_geometry::parse(function, normalization_root)? else {
            continue;
        };
        let key = (geometry.source.clone(), function.name.clone());
        let expected = report
            .remove(&key)
            .ok_or_else(|| format!("missing function report row for {}", function.name))?;
        let lines = line_coverage(&build_segments(&geometry.main_regions).map_err(invalid_map)?)
            .map_err(invalid_map)?
            .into_iter()
            .map(|line| line.count)
            .collect::<Vec<_>>();
        let regions = function
            .regions
            .iter()
            .filter(|region| region[crate::coverage_json::KIND_INDEX] == CODE_REGION_KIND)
            .map(|region| region[COUNT_INDEX])
            .collect::<Vec<_>>();
        let branches = function
            .branches
            .iter()
            .flat_map(|branch| {
                [
                    branch[COUNT_INDEX],
                    branch[crate::coverage_json::FALSE_COUNT_INDEX],
                ]
            })
            .collect::<Vec<_>>();
        validate_vectors(&lines, &regions, &branches, expected, &function.name)?;
        let identity = signature::identity(&function.name, &geometry.filenames)?;
        let instance = Instance {
            branch_found: expected.branches.found,
            column: geometry.column,
            identity: identity.clone(),
            line: geometry.line,
            name: function.name.clone(),
            signature: signature::mapping(function, expected.branches.found)?,
            source: geometry.source.clone(),
            vectors: BTreeMap::from([
                (MetricKind::Lines, lines),
                (MetricKind::Functions, vec![function.count]),
                (MetricKind::Regions, regions),
                (MetricKind::Branches, branches),
            ]),
        };
        let logical = (
            geometry.source.clone(),
            geometry.line,
            geometry.column,
            identity,
        );
        if !identities.insert(logical) {
            return Err(format!(
                "duplicate coverage instance {} in {}",
                function.name, geometry.source
            ));
        }
        instances.entry(geometry.source).or_default().push(instance);
    }
    if let Some(((source, name), _)) = report.into_iter().next() {
        return Err(format!(
            "function report row has no JSON function: {name} in {source}"
        ));
    }
    Ok(instances)
}

fn validate_vectors(
    lines: &[u64],
    regions: &[u64],
    branches: &[u64],
    expected: FunctionMetrics,
    name: &str,
) -> Result<(), String> {
    compare("line", lines, expected.lines, None, name)?;
    compare("region", regions, expected.regions, None, name)?;
    compare(
        "branch",
        branches,
        expected.branches,
        Some(expected.branches.found),
        name,
    )
}

fn compare(
    label: &str,
    counts: &[u64],
    expected: Metric,
    found: Option<u64>,
    name: &str,
) -> Result<(), String> {
    let actual = Metric {
        found: found.unwrap_or(u64::try_from(counts.len()).map_err(|_| "metric overflow")?),
        hit: u64::try_from(counts.iter().filter(|count| **count > 0).count())
            .map_err(|_| "covered metric overflow")?,
    }
    .validate(label)?;
    if actual != expected {
        return Err(format!(
            "{label} vector disagrees with function report for {name}"
        ));
    }
    Ok(())
}

fn invalid_map(error: impl std::fmt::Display) -> String {
    format!("invalid LLVM coverage map: {error}")
}
