use crate::constants::COVERAGE_MANIFEST_ENV;
use crate::coverage_json::{Data, SummaryMetric};
use crate::environment::Context;
use crate::model::{Metric, SummaryMap};
use crate::normalize;
use std::collections::BTreeMap;
use std::path::Path;
use yolo_coverage_protocol::{MetricKind, SOURCE_TAG, production_key};

const MISSING_MANIFEST: &str = "staged coverage source requires Bazel's source manifest";
const INVALID_UTF8: &str = "invalid staged coverage source manifest UTF-8";

pub(crate) fn reject_staged_sources(
    context: &Context,
    data: &Data,
    lcov: &str,
) -> Result<(), String> {
    let paths = data
        .files
        .iter()
        .map(|file| file.filename.as_str())
        .chain(
            data.functions
                .iter()
                .flat_map(|function| function.filenames.iter().map(String::as_str)),
        )
        .chain(
            lcov.lines()
                .filter_map(|line| line.strip_prefix(SOURCE_TAG)),
        )
        .map(|path| normalize::source(path, &context.execution_root));
    yolo_coverage_source_contract::reject_staged_production(paths, || {
        let path =
            std::env::var_os(COVERAGE_MANIFEST_ENV).ok_or_else(|| MISSING_MANIFEST.to_owned())?;
        let bytes = crate::declaration_sources::read_bounded(&context.execution_root.join(path))?;
        String::from_utf8(bytes).map_err(|error| format!("{INVALID_UTF8}: {error}"))
    })
}

pub fn summaries(data: &Data, execution_root: &Path) -> Result<SummaryMap, String> {
    let mut summaries = SummaryMap::new();
    for file in &data.files {
        let normalized = normalize::source(&file.filename, execution_root);
        let Some(source) = production_key(&normalized)? else {
            continue;
        };
        let metrics = BTreeMap::from([
            (
                MetricKind::Lines,
                metric(&file.summary.lines, "lines", &source)?,
            ),
            (
                MetricKind::Functions,
                metric(&file.summary.functions, "functions", &source)?,
            ),
            (
                MetricKind::Regions,
                metric(&file.summary.regions, "regions", &source)?,
            ),
            (
                MetricKind::Branches,
                metric(&file.summary.branches, "branches", &source)?,
            ),
        ]);
        if summaries.insert(source.clone(), metrics).is_some() {
            return Err(format!("duplicate llvm-cov summary for {source}"));
        }
    }
    if summaries.is_empty() {
        return Err("llvm-cov JSON contains no production sources".to_owned());
    }
    Ok(summaries)
}

pub fn sources(summaries: &SummaryMap) -> Vec<String> {
    summaries.keys().cloned().collect()
}

fn metric(value: &SummaryMetric, label: &str, source: &str) -> Result<Metric, String> {
    Metric {
        found: value.count,
        hit: value.covered,
    }
    .validate(&format!("{label} summary in {source}"))
}
