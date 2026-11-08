use crate::constants::{FUNCTION_REPORT_DIVIDER_SAMPLE, FUNCTION_REPORT_SHORT_DIVIDER_SAMPLE};
use crate::function_report::parse;
use crate::function_report_fields::{
    BRANCH_FOUND, BRANCH_MISSED, BRANCH_PERCENT, LINE_FOUND, LINE_MISSED, LINE_PERCENT,
    REGION_FOUND, REGION_MISSED, REGION_PERCENT,
};
use std::path::Path;

const ROOT: &str = "/tmp/function-report-root";
const SOURCE: &str = "build_system/model/src/lib.rs";
const OTHER_SOURCE: &str = "build_system/model/src/other.rs";
const HEADER: &str = "Name Regions Miss Cover Lines Miss Cover Branches Miss Cover";
const DIVIDER: &str = FUNCTION_REPORT_DIVIDER_SAMPLE;
const ROW: &str = "function 1 0 100% 1 0 100% 2 1 50%";
const TOTAL: &str = "TOTAL 1 0 100% 1 0 100% 2 1 50%";
const ZERO_TOTAL: &str = "TOTAL 0 0 0% 0 0 0% 0 0 0%";
const METRIC_COLUMNS: usize = 3;

fn section(source: &str, body: &[&str]) -> String {
    format!("File '{source}':\n{}\n", body.join("\n"))
}

fn complete(source: &str) -> String {
    section(source, &[HEADER, DIVIDER, ROW, DIVIDER, TOTAL])
}

fn rejected(text: &str, expected: &str) {
    let error = parse(text, Path::new(ROOT)).unwrap_err();
    assert!(error.contains(expected), "{error}; input={text:?}");
}

#[test]
fn empty_reports_and_content_outside_sections_are_rejected() {
    for text in ["", " \n\t\r\n"] {
        rejected(text, "no functions");
    }
    for text in [HEADER, DIVIDER, ROW, "File 'unfinished'", "File missing:"] {
        rejected(text, "outside a file");
    }
    for body in [vec![], vec![HEADER], vec![HEADER, DIVIDER, ROW]] {
        rejected(&section(SOURCE, &body), "incomplete");
    }
    rejected(&complete(""), "incomplete");
    rejected(
        &section(SOURCE, &[HEADER, DIVIDER, DIVIDER, ZERO_TOTAL]),
        "no functions",
    );
}

#[test]
fn headers_dividers_rows_and_totals_have_a_strict_order() {
    for body in [vec![HEADER, HEADER], vec![HEADER, DIVIDER, ROW, HEADER]] {
        rejected(&section(SOURCE, &body), "duplicate function report header");
    }
    for body in [
        vec![DIVIDER],
        vec![HEADER, DIVIDER, ROW, DIVIDER, DIVIDER],
        vec![HEADER, DIVIDER, ROW, DIVIDER, TOTAL, DIVIDER],
    ] {
        rejected(&section(SOURCE, &body), "invalid function report divider");
    }
    for body in [
        vec![ROW],
        vec![HEADER, ROW],
        vec![HEADER, FUNCTION_REPORT_SHORT_DIVIDER_SAMPLE],
        vec![HEADER, "-x-"],
        vec![HEADER, DIVIDER, ROW, DIVIDER, TOTAL, ROW],
    ] {
        rejected(&section(SOURCE, &body), "misordered function row");
    }
    rejected(
        &section(SOURCE, &[HEADER, DIVIDER, ROW, TOTAL]),
        "total precedes footer",
    );
    rejected(
        &section(SOURCE, &[HEADER, DIVIDER, ROW, DIVIDER, ROW]),
        "row follows footer",
    );
    rejected(
        &section(SOURCE, &[HEADER, DIVIDER, ROW, DIVIDER]),
        "missing function report total",
    );
}

#[test]
fn file_transitions_preserve_function_scope_and_reject_duplicate_sources() {
    let text = format!("{}\n{}", complete(SOURCE), complete(OTHER_SOURCE));
    let parsed = parse(&text, Path::new(ROOT)).unwrap();
    assert_eq!(parsed.len(), 2);
    assert!(parsed.contains_key(&(SOURCE.to_owned(), "function".to_owned())));
    assert!(parsed.contains_key(&(OTHER_SOURCE.to_owned(), "function".to_owned())));
    rejected(
        &format!("{}{}", complete(SOURCE), complete(SOURCE)),
        "duplicate function report source",
    );
    rejected(
        &format!("{}{}", section(SOURCE, &[HEADER]), complete(OTHER_SOURCE)),
        "incomplete",
    );
    let whitespace = complete(SOURCE).replace(HEADER, &HEADER.replace(' ', " \t "));
    assert!(parse(&whitespace, Path::new(ROOT)).is_ok());
}

#[test]
fn every_metric_rejects_sum_overflow_before_accepting_totals() {
    for slot in 0..METRIC_COLUMNS {
        let mut maximum = [
            "0 0 0%".to_owned(),
            "0 0 0%".to_owned(),
            "0 0 0%".to_owned(),
        ];
        let mut increment = maximum.clone();
        maximum[slot] = format!("{} 0 100%", u64::MAX);
        increment[slot] = "1 0 100%".to_owned();
        let first = format!("maximum {}", maximum.join(" "));
        let second = format!("increment {}", increment.join(" "));
        rejected(
            &section(
                SOURCE,
                &[HEADER, DIVIDER, &first, &second, DIVIDER, ZERO_TOTAL],
            ),
            "overflow",
        );
    }
}

#[test]
fn malformed_field_counts_and_each_numeric_column_fail_closed() {
    for row in [
        "function",
        "function 1 0 100% 1 0 100% 2 1",
        "function 1 0 100% 1 0 100% 2 1 50% extra",
    ] {
        assert!(parse(&section(SOURCE, &[HEADER, DIVIDER, row]), Path::new(ROOT)).is_err());
    }
    for index in [
        REGION_FOUND,
        REGION_MISSED,
        LINE_FOUND,
        LINE_MISSED,
        BRANCH_FOUND,
        BRANCH_MISSED,
    ] {
        let mut fields = ROW.split_whitespace().collect::<Vec<_>>();
        fields[index] = "invalid";
        assert!(
            parse(
                &section(SOURCE, &[HEADER, DIVIDER, &fields.join(" ")]),
                Path::new(ROOT)
            )
            .is_err()
        );
    }
    for index in [REGION_PERCENT, LINE_PERCENT, BRANCH_PERCENT] {
        for value in ["50", "-1%", "101%", "NaN%", "inf%"] {
            let mut fields = ROW.split_whitespace().collect::<Vec<_>>();
            fields[index] = value;
            assert!(
                parse(
                    &section(SOURCE, &[HEADER, DIVIDER, &fields.join(" ")]),
                    Path::new(ROOT)
                )
                .is_err()
            );
        }
    }
}
