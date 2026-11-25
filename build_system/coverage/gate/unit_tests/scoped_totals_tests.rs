use crate::lcov::{parse, parse_scope};
use crate::lcov_fixture::{self, Decorations};
use crate::scope_fixture::*;
use yolo_coverage_protocol::ProductionScope;

#[test]
fn mixed_report_totals_never_dilute_the_selected_scope() {
    for (uncovered, covered, failed_scope, passed_scope) in [
        (
            CORE_SOURCE,
            TOOLING_SOURCE,
            ProductionScope::Core,
            ProductionScope::Tooling,
        ),
        (
            TOOLING_SOURCE,
            CORE_SOURCE,
            ProductionScope::Tooling,
            ProductionScope::Core,
        ),
    ] {
        let mixed = format!(
            "{}{}",
            report(uncovered, SINGLE_INSTANCE, false),
            report(covered, LARGE_INSTANCE_COUNT, true),
        );
        let failed = parse_scope(&mixed, failed_scope).unwrap();
        let passed = parse_scope(&mixed, passed_scope).unwrap();
        assert!(!failed.passes());
        assert!(passed.passes());
        for (_, metric) in failed.metrics() {
            assert_eq!(metric.found, SINGLE_INSTANCE as u64);
            assert_eq!(metric.hit, 0);
        }
        assert_eq!(parse(&mixed), parse_scope(&mixed, ProductionScope::Core));
    }
}

#[test]
fn shared_inventories_are_validated_before_scope_selection() {
    let core = shared_report(CORE_SOURCE, true);
    let tooling = shared_report(TOOLING_SOURCE, true);
    for scope in [ProductionScope::Core, ProductionScope::Tooling] {
        assert!(
            parse_scope(&format!("{core}{tooling}"), scope)
                .unwrap()
                .passes()
        );
        assert!(parse_scope(&core, scope).is_err());
        assert!(parse_scope(&tooling, scope).is_err());
    }
}

#[test]
fn ignored_scope_cannot_hide_missing_markers_or_inventories() {
    let core = report(CORE_SOURCE, SINGLE_INSTANCE, true);
    let entries = entries(SINGLE_INSTANCE, true);
    for decorations in [
        Decorations::None,
        Decorations::MarkerOnly,
        Decorations::MarkerAndSourceInventory,
    ] {
        let tooling = lcov_fixture::report(TOOLING_SOURCE, &entries, &entries, decorations);
        assert!(parse(&format!("{core}{tooling}")).is_err());
    }
}

#[test]
fn ignored_scope_still_validates_metric_vectors() {
    let core = report(CORE_SOURCE, SINGLE_INSTANCE, true);
    let inventory = entries(SINGLE_INSTANCE, true);
    let mut broken = inventory.clone();
    broken.pop();
    let tooling = lcov_fixture::report(TOOLING_SOURCE, &broken, &inventory, Decorations::Complete);
    assert!(parse(&format!("{core}{tooling}")).is_err());
}

#[test]
fn selecting_an_absent_or_uninstrumentable_scope_fails_closed() {
    let core = report(CORE_SOURCE, SINGLE_INSTANCE, true);
    let tooling = report(TOOLING_SOURCE, SINGLE_INSTANCE, true);
    assert!(parse_scope(&core, ProductionScope::Tooling).is_err());
    assert!(parse(&tooling).is_err());
    let empty = format!("SF:{TOOLING_SOURCE}\nFNF:0\nFNH:0\nLF:0\nLH:0\nend_of_record\n");
    assert!(parse_scope(&format!("{core}{empty}"), ProductionScope::Tooling).is_err());
}
