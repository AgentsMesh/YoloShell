use super::*;
use yolo_coverage_protocol::{FunctionToken, MINIMUM_SOURCE_POSITION, parse_token};

const OUTSIDE_SOURCE: &str = "third_party/fixture.rs";
const NEXT_POSITION: u64 = 1;

fn baseline() -> String {
    report(&instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1), true)
}

fn rejects(report: &str, expected: &str) {
    let error = parse(report).unwrap_err();
    assert!(
        error.contains(expected),
        "expected {expected:?}, got {error:?}"
    );
}

#[test]
fn framing_rejects_orphan_nested_empty_and_truncated_records() {
    for (input, expected) in [
        ("SF:\n", "invalid nested LCOV source"),
        (
            "SF:core/src/a.rs\nSF:core/src/b.rs\n",
            "invalid nested LCOV source",
        ),
        ("end_of_record\n", "record end without source"),
        ("DA:1,1\n", "LCOV content outside a record"),
        ("SF:core/src/a.rs\n", "unterminated LCOV source"),
        ("TN:fixture\n\n", "contains no production sources"),
    ] {
        rejects(input, expected);
    }
}

#[test]
fn windows_line_endings_test_names_and_ordinary_dependency_records_preserve_totals() {
    let report = baseline();
    let expected = parse(&report).unwrap();
    let report = format!(
        "TN:fixture\n\n{report}SF:{OUTSIDE_SOURCE}\nFN:1,ordinary\nFNDA:1,ordinary\nend_of_record\n"
    );
    assert_eq!(parse(&report.replace('\n', "\r\n")).unwrap(), expected);
}

#[test]
fn repeated_production_records_cannot_double_count_coverage() {
    let report = baseline();
    rejects(
        &format!("{report}{report}"),
        "duplicate production LCOV source",
    );
}

#[test]
fn synthetic_tokens_cannot_move_to_nonproduction_or_another_source_position() {
    let marker = [(MINIMUM_SOURCE_POSITION, MARKER_TOKEN.to_owned(), 1)];
    let report = crate::lcov_fixture::report(OUTSIDE_SOURCE, &marker, &[], Decorations::None);
    rejects(
        &report,
        "synthetic function token appears outside production Rust",
    );
    let misplaced = [(
        MINIMUM_SOURCE_POSITION + NEXT_POSITION,
        MARKER_TOKEN.to_owned(),
        1,
    )];
    rejects(
        &super::report(&misplaced, false),
        "differs from token location",
    );
}

#[test]
fn malformed_synthetic_definition_and_count_fields_fail_closed() {
    for (entry, expected) in [
        ("FN:missing_separator".to_owned(), "malformed FN entry"),
        ("FNDA:missing_separator".to_owned(), "malformed FNDA entry"),
        (
            format!("FN:invalid,{MARKER_TOKEN}"),
            "invalid synthetic function line",
        ),
        (
            format!("FNDA:invalid,{MARKER_TOKEN}"),
            "invalid synthetic function count",
        ),
        (
            format!("DA:1,{MARKER_TOKEN}"),
            "synthetic token appears in an invalid LCOV line",
        ),
    ] {
        rejects(&format!("SF:{SOURCE}\n{entry}\nend_of_record\n"), expected);
    }
}

#[test]
fn zero_count_marker_and_headers_cannot_assert_presence() {
    let complete = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    let mut invalid = complete.clone();
    invalid.push((MINIMUM_SOURCE_POSITION, MARKER_TOKEN.to_owned(), 0));
    rejects(
        &report_with_inventory(&invalid, &complete, false),
        "coverage marker multiplicity is outside",
    );
    let mut invalid = complete.clone();
    invalid[0].2 = 0;
    rejects(
        &report_with_inventory(&invalid, &complete, true),
        "synthetic metric header multiplicity is outside",
    );
}

#[test]
fn metric_vectors_reject_missing_entries_holes_and_invalid_function_shape() {
    let complete = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    for kind in MetricKind::ALL {
        let mut missing = complete.clone();
        missing.retain(|(_, name, _)| {
            !matches!(parse_token(name).unwrap(), Some(FunctionToken::Ordinal { kind: actual, .. }) if actual == kind)
        });
        rejects(
            &report_with_inventory(&missing, &complete, true),
            "vector length differs from its header",
        );
        let mut hole = complete.clone();
        for (_, name, _) in &mut hole {
            if matches!(parse_token(name).unwrap(), Some(FunctionToken::Ordinal { kind: actual, .. }) if actual == kind)
            {
                *name = ordinal_token(
                    kind,
                    GROUP_LINE,
                    GROUP_COLUMN,
                    ID_A,
                    SIGNATURE,
                    NEXT_POSITION,
                );
            }
        }
        rejects(
            &report_with_inventory(&hole, &complete, true),
            "ordinals are not contiguous",
        );
    }
    let invalid = instance(ID_A, SIGNATURE, &[1], &[1, 1], &[1], &[1], 1);
    rejects(
        &report(&invalid, true),
        "function vector must contain exactly one ordinal",
    );
}

#[test]
fn conflicting_headers_cannot_overwrite_one_instances_denominator() {
    let complete = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    let mut conflicting = complete.clone();
    const CONFLICTING_LINE_COUNT: u64 = 2;
    conflicting.push((
        GROUP_LINE,
        header_token(
            MetricKind::Lines,
            GROUP_LINE,
            GROUP_COLUMN,
            ID_A,
            SIGNATURE,
            CONFLICTING_LINE_COUNT,
            CONFLICTING_LINE_COUNT,
        ),
        1,
    ));
    rejects(
        &report_with_inventory(&conflicting, &complete, true),
        "duplicate synthetic metric header",
    );
}

#[test]
fn a_zero_function_denominator_cannot_hide_a_real_function_ordinal() {
    let mut entries = instance(ID_A, SIGNATURE, &[1], &[0], &[1], &[1], 1);
    let zero_header = metric(MetricKind::Functions, ID_A, SIGNATURE, &[0], Some(0))
        .into_iter()
        .next()
        .unwrap()
        .1;
    for (_, name, _) in &mut entries {
        if matches!(
            parse_token(name).unwrap(),
            Some(FunctionToken::Header {
                kind: MetricKind::Functions,
                ..
            })
        ) {
            *name = zero_header.clone();
        }
    }
    rejects(
        &report(&entries, true),
        "function vector must contain exactly one ordinal",
    );
}
