use crate::constants::ORDINAL_MULTIPLICITY_ERROR;
use crate::lcov::{InstanceRecords, ParsedReport};
use crate::metrics::{Metric, Totals};
use std::collections::BTreeMap;
use yolo_coverage_protocol::{MetricKind, ProductionScope, production_scope};

#[derive(Clone, Copy, Default)]
struct MetricSet {
    branches: Metric,
    functions: Metric,
    lines: Metric,
    regions: Metric,
}

impl MetricSet {
    fn get(self, kind: MetricKind) -> Metric {
        match kind {
            MetricKind::Lines => self.lines,
            MetricKind::Functions => self.functions,
            MetricKind::Regions => self.regions,
            MetricKind::Branches => self.branches,
        }
    }

    fn set(&mut self, kind: MetricKind, metric: Metric) {
        match kind {
            MetricKind::Lines => self.lines = metric,
            MetricKind::Functions => self.functions = metric,
            MetricKind::Regions => self.regions = metric,
            MetricKind::Branches => self.branches = metric,
        }
    }

    fn into_totals(self) -> Result<Totals, String> {
        Totals::new(self.lines, self.functions, self.regions, self.branches)
    }
}

pub fn calculate(report: ParsedReport, scope: ProductionScope) -> Result<Totals, String> {
    if report.production.is_empty() {
        return Err("LCOV report contains no production sources".to_owned());
    }
    crate::lcov_inventory::validate(&report)?;
    let mut groups = BTreeMap::<(String, u64, u64), MetricSet>::new();
    for (key, records) in report.instances {
        if !report.markers.contains_key(&key.source) {
            return Err(format!(
                "synthetic instance has no marker for {}",
                key.source
            ));
        }
        let metrics = instance_metrics(&records)?;
        let group = groups
            .entry((key.source, key.line, key.column))
            .or_default();
        for kind in MetricKind::ALL {
            let current = group.get(kind);
            let candidate = metrics.get(kind);
            group.set(
                kind,
                Metric {
                    found: current.found.max(candidate.found),
                    hit: current.hit.max(candidate.hit),
                },
            );
        }
    }
    let total =
        groups
            .into_iter()
            .try_fold(MetricSet::default(), |sum, ((source, _, _), group)| {
                if production_scope(&source)? != Some(scope) {
                    return Ok(sum);
                }
                Ok::<_, String>(MetricSet {
                    lines: sum
                        .lines
                        .checked_add(group.lines, MetricKind::Lines.label())?,
                    functions: sum
                        .functions
                        .checked_add(group.functions, MetricKind::Functions.label())?,
                    regions: sum
                        .regions
                        .checked_add(group.regions, MetricKind::Regions.label())?,
                    branches: sum
                        .branches
                        .checked_add(group.branches, MetricKind::Branches.label())?,
                })
            })?;
    total.into_totals()
}

fn instance_metrics(records: &InstanceRecords) -> Result<MetricSet, String> {
    if records.headers.len() != MetricKind::ALL.len()
        || records.ordinals.len() != MetricKind::ALL.len()
    {
        return Err("synthetic instance does not contain every metric".to_owned());
    }
    let mut metrics = MetricSet::default();
    for kind in MetricKind::ALL {
        let header = records
            .headers
            .get(&kind)
            .ok_or_else(|| format!("synthetic instance has no {} header", kind.label()))?;
        let ordinals = records
            .ordinals
            .get(&kind)
            .ok_or_else(|| format!("synthetic instance has no {} ordinals", kind.label()))?;
        if kind == MetricKind::Functions && (header.vector_len != 1 || header.found != 1) {
            return Err("function vector must contain exactly one ordinal".to_owned());
        }
        let vector_len = u64::try_from(ordinals.len())
            .map_err(|_| format!("{} denominator overflow", kind.label()))?;
        if vector_len != header.vector_len {
            return Err(format!(
                "{} vector length differs from its header",
                kind.label()
            ));
        }
        for (expected, ordinal) in ordinals.keys().copied().enumerate() {
            let expected = u64::try_from(expected).map_err(|_| "metric ordinal overflow")?;
            if ordinal != expected {
                return Err(format!("{} ordinals are not contiguous", kind.label()));
            }
        }
        if ordinals.values().any(|count| *count > header.count) {
            return Err(ORDINAL_MULTIPLICITY_ERROR.to_owned());
        }
        let hit = u64::try_from(ordinals.values().filter(|count| **count > 0).count())
            .map_err(|_| format!("{} covered count overflow", kind.label()))?;
        metrics.set(kind, Metric::new(header.found, hit, kind.label())?);
    }
    Ok(metrics)
}
