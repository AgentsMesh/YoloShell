use crate::constants::FUNCTION_REPORT_DIVIDER_SAMPLE;
use crate::function_report::parse;
use crate::model::Metric;
use std::path::Path;

const ROOT: &str = "/tmp/execroot/_main";
const SOURCE: &str = "core/src/lib.rs";
const HEADER: &str = "Name Regions Miss Cover Lines Miss Cover Branches Miss Cover";

#[test]
fn exact_per_instance_metrics_are_parsed() {
    let text = report(&[
        "instance_a 6 1 83.33% 3 0 100.00% 2 1 50.00%",
        "instance_b 5 5 0.00% 4 2 50.00% 0 0 0.00%",
        "TOTAL 11 6 45.45% 7 2 71.43% 2 1 50.00%",
    ]);
    let parsed = parse(&text, Path::new(ROOT)).unwrap();
    let metrics = parsed
        .get(&(SOURCE.to_owned(), "instance_a".to_owned()))
        .unwrap();
    assert_eq!(metrics.regions, Metric { found: 6, hit: 5 });
    assert_eq!(metrics.lines, Metric { found: 3, hit: 3 });
    assert_eq!(metrics.branches, Metric { found: 2, hit: 1 });
}

#[test]
fn duplicate_rows_missing_totals_and_bad_counts_fail_closed() {
    for rows in [
        vec![
            "same 1 0 100.00% 1 0 100.00% 0 0 0.00%",
            "same 1 0 100.00% 1 0 100.00% 0 0 0.00%",
            "TOTAL 2 0 100.00% 2 0 100.00% 0 0 0.00%",
        ],
        vec!["one 1 0 100.00% 1 0 100.00% 0 0 0.00%"],
        vec![
            "one 1 2 100.00% 1 0 100.00% 0 0 0.00%",
            "TOTAL 1 2 100.00% 1 0 100.00% 0 0 0.00%",
        ],
        vec![
            "one 1 0 nope 1 0 100.00% 0 0 0.00%",
            "TOTAL 1 0 100.00% 1 0 100.00% 0 0 0.00%",
        ],
    ] {
        assert!(parse(&report(&rows), Path::new(ROOT)).is_err(), "{rows:?}");
    }
}

#[test]
fn reported_total_must_equal_the_rows() {
    let text = report(&[
        "one 1 0 100.00% 1 0 100.00% 0 0 0.00%",
        "TOTAL 2 0 100.00% 1 0 100.00% 0 0 0.00%",
    ]);
    assert!(parse(&text, Path::new(ROOT)).is_err());
}

fn report(rows: &[&str]) -> String {
    let split = rows
        .iter()
        .position(|row| row.starts_with("TOTAL "))
        .unwrap_or(rows.len());
    let (functions, total) = rows.split_at(split);
    let footer = (!total.is_empty())
        .then(|| format!("{FUNCTION_REPORT_DIVIDER_SAMPLE}\n{}\n", total.join("\n")))
        .unwrap_or_default();
    format!(
        "File '{SOURCE}':\n{HEADER}\n{FUNCTION_REPORT_DIVIDER_SAMPLE}\n{}\n{footer}",
        functions.join("\n")
    )
}
