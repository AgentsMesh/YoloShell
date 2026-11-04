use crate::model::Metric;
use crate::normalize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const FILE_PREFIX: &str = "File '";
const FILE_SUFFIX: &str = "':";
const HEADER: &str = "Name Regions Miss Cover Lines Miss Cover Branches Miss Cover";
const TOTAL_LABEL: &str = "TOTAL";
const MINIMUM_DIVIDER_LENGTH: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FunctionMetrics {
    pub branches: Metric,
    pub lines: Metric,
    pub regions: Metric,
}

pub type FunctionReport = BTreeMap<(String, String), FunctionMetrics>;

#[derive(Default)]
struct Section {
    divider: bool,
    footer: bool,
    header: bool,
    rows: BTreeMap<String, FunctionMetrics>,
    source: String,
    total: Option<FunctionMetrics>,
}

pub fn parse(text: &str, execution_root: &Path) -> Result<FunctionReport, String> {
    let mut report = FunctionReport::new();
    let mut sources = BTreeSet::new();
    let mut current: Option<Section> = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(source) = file_header(line) {
            if let Some(section) = current.take() {
                finish(section, &mut report, &mut sources)?;
            }
            current = Some(Section {
                source: normalize::source(source, execution_root),
                ..Section::default()
            });
            continue;
        }
        let section = current
            .as_mut()
            .ok_or_else(|| format!("function report content outside a file: {line:?}"))?;
        if line.split_whitespace().collect::<Vec<_>>().join(" ") == HEADER {
            if section.header {
                return Err(format!(
                    "duplicate function report header in {}",
                    section.source
                ));
            }
            section.header = true;
        } else if divider(line) {
            if !section.header || section.total.is_some() || section.footer {
                return Err(format!(
                    "invalid function report divider in {}",
                    section.source
                ));
            }
            if section.divider {
                section.footer = true;
            } else {
                section.divider = true;
            }
        } else {
            parse_row(line, section)?;
        }
    }
    if let Some(section) = current {
        finish(section, &mut report, &mut sources)?;
    }
    if report.is_empty() {
        return Err("function report contains no functions".to_owned());
    }
    Ok(report)
}

fn file_header(line: &str) -> Option<&str> {
    line.strip_prefix(FILE_PREFIX)?.strip_suffix(FILE_SUFFIX)
}

fn divider(line: &str) -> bool {
    line.len() >= MINIMUM_DIVIDER_LENGTH && line.bytes().all(|byte| byte == b'-')
}

fn parse_row(line: &str, section: &mut Section) -> Result<(), String> {
    if !section.header || !section.divider || section.total.is_some() {
        return Err(format!("misordered function row in {}", section.source));
    }
    let fields = line.split_whitespace().collect::<Vec<_>>();
    let metrics = crate::function_report_fields::parse(&fields, &section.source)?;
    if fields[0] == TOTAL_LABEL {
        if !section.footer {
            return Err(format!(
                "function report total precedes footer in {}",
                section.source
            ));
        }
        section.total = Some(metrics);
    } else if section.footer {
        return Err(format!("function row follows footer in {}", section.source));
    } else if section.rows.insert(fields[0].to_owned(), metrics).is_some() {
        return Err(format!(
            "duplicate function {:?} in {}",
            fields[0], section.source
        ));
    }
    Ok(())
}

fn finish(
    section: Section,
    report: &mut FunctionReport,
    sources: &mut BTreeSet<String>,
) -> Result<(), String> {
    if section.source.is_empty() || !section.header || !section.divider || !section.footer {
        return Err("incomplete function report section".to_owned());
    }
    if !sources.insert(section.source.clone()) {
        return Err(format!(
            "duplicate function report source {}",
            section.source
        ));
    }
    let expected = section
        .total
        .ok_or_else(|| format!("missing function report total in {}", section.source))?;
    let actual = section.rows.values().try_fold(
        FunctionMetrics {
            branches: Metric::default(),
            lines: Metric::default(),
            regions: Metric::default(),
        },
        |total, row| {
            Ok::<_, String>(FunctionMetrics {
                branches: total.branches.checked_add(row.branches, "branch report")?,
                lines: total.lines.checked_add(row.lines, "line report")?,
                regions: total.regions.checked_add(row.regions, "region report")?,
            })
        },
    )?;
    if actual != expected {
        return Err(format!(
            "function report total mismatch in {}",
            section.source
        ));
    }
    for (name, metrics) in section.rows {
        report.insert((section.source.clone(), name), metrics);
    }
    Ok(())
}
