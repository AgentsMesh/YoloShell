use crate::gate_cli::{execute, stdout};
use crate::multiplicity_fixture::*;
use crate::scope_fixture::{CORE_SOURCE, TOOLING_SOURCE};
use yolo_coverage_protocol::{CORE_SCOPE_ARGUMENT, MetricKind, TOOLING_SCOPE_ARGUMENT};

#[test]
fn genuine_additive_merge_remains_accepted_by_both_cli_scopes() {
    let report = format!("{}{}", merged(CORE_SOURCE), merged(TOOLING_SOURCE));
    for scope in [
        None,
        Some(CORE_SCOPE_ARGUMENT),
        Some(TOOLING_SCOPE_ARGUMENT),
    ] {
        let result = execute(&report, scope);
        assert!(result.status.success());
        assert!(!stdout(&result).is_empty());
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn impossible_ordinals_in_the_unselected_scope_fail_before_cli_totals_are_printed() {
    for (selected, hidden, scope) in [
        (CORE_SOURCE, TOOLING_SOURCE, None),
        (CORE_SOURCE, TOOLING_SOURCE, Some(CORE_SCOPE_ARGUMENT)),
        (TOOLING_SOURCE, CORE_SOURCE, Some(TOOLING_SCOPE_ARGUMENT)),
    ] {
        let healthy = merged(selected);
        let baseline = merged(hidden);
        for kind in MetricKind::ALL {
            for count in [MERGED_REPORT_COUNT + COUNT_STEP, u64::MAX] {
                let corrupt = with_ordinal_count(&baseline, kind, count);
                let report = format!("{healthy}{corrupt}");
                let result = execute(&report, scope);
                assert_eq!(result.status.code(), Some(FAILING_EXIT_CODE));
                assert!(stdout(&result).is_empty());
                let error = std::str::from_utf8(&result.stderr).unwrap();
                assert!(error.contains(INVALID_MULTIPLICITY));
            }
        }
    }
}
