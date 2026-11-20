#[path = "../test_support/gate_cli.rs"]
mod gate_cli;
#[path = "../test_support/lcov_fixture.rs"]
mod lcov_fixture;
#[path = "../test_support/multiplicity_fixture.rs"]
mod multiplicity_fixture;
mod ordinal_multiplicity;
#[path = "../test_support/scope_fixture.rs"]
mod scope_fixture;

use gate_cli::{execute, stdout};
use scope_fixture::*;

use yolo_coverage_protocol::TOOLING_SCOPE_ARGUMENT;
const CORE_TITLE: &str = "Rust production coverage";
const TOOLING_TITLE: &str = "Rust build-system production coverage";
const METRICS: [&str; 4] = ["Lines", "Functions", "Regions", "Branches"];
const FAILING_EXIT_CODE: i32 = 1;

#[test]
fn default_gate_preserves_core_threshold_and_tooling_is_independently_reported() {
    let mixed = format!(
        "{}{}",
        report(CORE_SOURCE, SINGLE_INSTANCE, false),
        report(TOOLING_SOURCE, LARGE_INSTANCE_COUNT, true),
    );
    let core = execute(&mixed, None);
    assert_eq!(core.status.code(), Some(FAILING_EXIT_CODE));
    assert!(stdout(&core).starts_with(CORE_TITLE));
    for metric in METRICS {
        assert!(stdout(&core).contains(&format!("{metric}: 0.00% (0/1)")));
    }
    let tooling = execute(&mixed, Some(TOOLING_SCOPE_ARGUMENT));
    assert!(tooling.status.success());
    assert!(stdout(&tooling).starts_with(TOOLING_TITLE));
    for metric in METRICS {
        assert!(stdout(&tooling).contains(&format!("{metric}: 100.00% (100/100)")));
    }
}

#[test]
fn tooling_failure_prints_every_measured_metric_without_failing_healthy_core() {
    let mixed = format!(
        "{}{}",
        report(CORE_SOURCE, SINGLE_INSTANCE, true),
        report(TOOLING_SOURCE, SINGLE_INSTANCE, false),
    );
    assert!(execute(&mixed, None).status.success());
    let tooling = execute(&mixed, Some(TOOLING_SCOPE_ARGUMENT));
    assert_eq!(tooling.status.code(), Some(FAILING_EXIT_CODE));
    for metric in METRICS {
        assert!(stdout(&tooling).contains(&format!("{metric}: 0.00% (0/1)")));
    }
}

#[test]
fn missing_cross_scope_inventory_member_rejects_both_gate_modes() {
    let incomplete = shared_report(CORE_SOURCE, true);
    for scope in [None, Some(TOOLING_SCOPE_ARGUMENT)] {
        let output = execute(&incomplete, scope);
        assert_eq!(output.status.code(), Some(FAILING_EXIT_CODE));
        assert!(stdout(&output).is_empty());
    }
}

#[test]
fn unselected_tooling_scope_cannot_bypass_any_inventory_layer() {
    let core = report(CORE_SOURCE, SINGLE_INSTANCE, true);
    let entries = entries(SINGLE_INSTANCE, true);
    for decorations in [
        lcov_fixture::Decorations::None,
        lcov_fixture::Decorations::MarkerOnly,
        lcov_fixture::Decorations::MarkerAndSourceInventory,
    ] {
        let tooling = lcov_fixture::report(TOOLING_SOURCE, &entries, &entries, decorations);
        let output = execute(&format!("{core}{tooling}"), None);
        assert_eq!(output.status.code(), Some(FAILING_EXIT_CODE));
        assert!(stdout(&output).is_empty());
    }
}

#[test]
fn tooling_only_reports_require_explicit_scope_and_nonempty_measurement() {
    let tooling = report(TOOLING_SOURCE, SINGLE_INSTANCE, true);
    assert!(
        execute(&tooling, Some(TOOLING_SCOPE_ARGUMENT))
            .status
            .success()
    );
    assert_eq!(
        execute(&tooling, None).status.code(),
        Some(FAILING_EXIT_CODE)
    );
    let core = report(CORE_SOURCE, SINGLE_INSTANCE, true);
    assert_eq!(
        execute(&core, Some(TOOLING_SCOPE_ARGUMENT)).status.code(),
        Some(FAILING_EXIT_CODE)
    );
}
