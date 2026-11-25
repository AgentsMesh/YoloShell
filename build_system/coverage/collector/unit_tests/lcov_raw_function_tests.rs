use super::*;
use yolo_coverage_protocol::{FUNCTIONS_FOUND_TAG, FUNCTIONS_HIT_TAG, TOKEN_ROOT};

fn total(report: &str, tag: &str) -> u64 {
    let values = report
        .lines()
        .filter_map(|line| line.strip_prefix(tag))
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 1);
    values[0].parse().unwrap()
}

#[test]
fn raw_function_summaries_are_rebuilt_without_changing_synthetic_metrics() {
    let baseline = augment(
        &format!("SF:{SOURCE}\nend_of_record\n"),
        &instances(),
        &sources(),
    )
    .unwrap();
    let raw = format!(
        "SF:{SOURCE}\nFN:1,hit\nFN:2,miss\nFNDA:3,hit\nFNDA:0,miss\nFNF:99\nFNH:98\nend_of_record\n"
    );
    let actual = augment(&raw, &instances(), &sources()).unwrap();
    assert!(actual.contains("FNDA:3,hit\nFNDA:0,miss\n"));
    assert_eq!(
        total(&actual, FUNCTIONS_FOUND_TAG),
        total(&baseline, FUNCTIONS_FOUND_TAG) + 2
    );
    assert_eq!(
        total(&actual, FUNCTIONS_HIT_TAG),
        total(&baseline, FUNCTIONS_HIT_TAG) + 1
    );
    for line in baseline.lines().filter(|line| line.contains(TOKEN_ROOT)) {
        assert!(actual.lines().any(|candidate| candidate == line));
    }
}

#[test]
fn malformed_raw_function_counts_are_not_silently_dropped() {
    for (entry, expected) in [
        ("FNDA:1", "malformed raw FNDA"),
        ("FNDA:invalid,raw", "invalid raw FNDA count"),
        ("FNDA:-1,raw", "invalid raw FNDA count"),
        ("FNDA:18446744073709551616,raw", "invalid raw FNDA count"),
    ] {
        let raw = format!("SF:{SOURCE}\n{entry}\nend_of_record\n");
        let error = augment(&raw, &instances(), &sources()).unwrap_err();
        assert!(error.contains(expected), "{entry}: {error}");
    }
}
