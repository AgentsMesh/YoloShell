use crate::scope_fixture::{self, SINGLE_INSTANCE};
use std::collections::BTreeMap;
use yolo_coverage_protocol::{FUNCTION_DATA_TAG, FunctionToken, MetricKind, parse_token};

#[path = "multiplicity_constants.rs"]
mod constants;
pub use constants::*;

pub fn merge(left: &str, right: &str) -> String {
    let mut remaining = right.lines().filter_map(data).collect::<BTreeMap<_, _>>();
    let mut output = String::new();
    for line in left.lines() {
        if let Some((name, count)) = data(line) {
            let sum = count.checked_add(remaining.remove(name).unwrap()).unwrap();
            output.push_str(&format!("{FUNCTION_DATA_TAG}{sum},{name}\n"));
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    assert!(remaining.is_empty());
    output
}

pub fn merged(source: &str) -> String {
    let single = scope_fixture::report(source, SINGLE_INSTANCE, true);
    merge(&single, &single)
}

pub fn with_ordinal_count(report: &str, kind: MetricKind, count: u64) -> String {
    let mut changed = usize::default();
    let mut output = String::new();
    for line in report.lines() {
        if let Some((name, _)) = data(line) {
            if matches!(
                parse_token(name).unwrap(),
                Some(FunctionToken::Ordinal { kind: actual, .. }) if actual == kind
            ) {
                output.push_str(&format!("{FUNCTION_DATA_TAG}{count},{name}\n"));
                changed += ONE_ORDINAL;
                continue;
            }
        }
        output.push_str(line);
        output.push('\n');
    }
    assert_eq!(changed, ONE_ORDINAL);
    output
}

fn data(line: &str) -> Option<(&str, u64)> {
    let (count, name) = line
        .strip_prefix(FUNCTION_DATA_TAG)?
        .split_once(',')
        .unwrap();
    Some((name, count.parse().unwrap()))
}
