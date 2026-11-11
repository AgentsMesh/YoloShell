mod arguments;
mod constants;
mod instance_descriptor;
mod instance_inventory;
#[cfg(test)]
#[path = "../unit_tests/instance_inventory_tests.rs"]
mod instance_inventory_tests;
mod lcov;
mod lcov_finish;
mod lcov_inventory;
mod lcov_record;
mod lcov_totals;
mod metrics;
mod source;
mod source_inventory;
#[cfg(test)]
#[path = "../unit_tests/source_inventory_tests.rs"]
mod source_inventory_tests;

#[cfg(test)]
#[path = "../unit_tests/arguments_tests.rs"]
mod arguments_tests;
#[cfg(test)]
#[path = "../unit_tests/gate_tests.rs"]
mod gate_tests;
#[cfg(test)]
#[path = "../unit_tests/lcov_adversarial_tests.rs"]
mod lcov_adversarial_tests;
#[cfg(test)]
#[path = "../test_support/lcov_fixture.rs"]
mod lcov_fixture;
#[cfg(test)]
#[path = "../unit_tests/lcov_tests.rs"]
mod lcov_tests;
#[cfg(test)]
#[path = "../unit_tests/metrics_tests.rs"]
mod metrics_tests;
#[cfg(test)]
#[path = "../test_support/multiplicity_fixture.rs"]
mod multiplicity_fixture;
#[cfg(test)]
#[path = "../unit_tests/ordinal_multiplicity_tests.rs"]
mod ordinal_multiplicity_tests;
#[cfg(test)]
#[path = "../test_support/scope_fixture.rs"]
mod scope_fixture;
#[cfg(test)]
#[path = "../unit_tests/scoped_totals_tests.rs"]
mod scoped_totals_tests;
#[cfg(test)]
#[path = "../unit_tests/source_tests.rs"]
mod source_tests;

use constants::*;
use metrics::{Metric, Totals};
use std::process::ExitCode;
use yolo_coverage_protocol::ProductionScope;

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{ERROR_PREFIX}{error}");
            ExitCode::FAILURE
        }
    }
}

fn execute() -> Result<(), String> {
    let arguments = arguments::load()?;
    let path = arguments.path;
    let report = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let totals = match arguments.scope {
        ProductionScope::Core => lcov::parse(&report)?,
        scope => lcov::parse_scope(&report, scope)?,
    };
    print_totals(totals, arguments.scope);
    if totals.passes() {
        Ok(())
    } else {
        Err(format!(
            "line, function, region, and branch coverage must each be strictly greater than {REQUIRED_PERCENT}%"
        ))
    }
}

fn print_totals(totals: Totals, scope: ProductionScope) {
    let title = match scope {
        ProductionScope::Core => REPORT_TITLE,
        ProductionScope::Tooling => TOOLING_REPORT_TITLE,
    };
    println!("{title}");
    for (label, metric) in totals.metrics() {
        print_metric(label, metric);
    }
}

fn print_metric(label: &str, metric: Metric) {
    let percent = metric.percentage();
    println!(
        "{label}: {percent:.precision$}% ({}/{})",
        metric.hit,
        metric.found,
        precision = PERCENT_DECIMAL_PLACES,
    );
}
