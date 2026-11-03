#[path = "source_regions.rs"]
mod source_regions;

use crate::constants::*;
use serde_json::Value;

pub fn check(report: &Value, variant: &str) {
    let functions = report[DATA]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|data| data[FUNCTIONS].as_array().unwrap())
        .collect::<Vec<_>>();
    zero_helpers(&functions, variant);
    caller_mappings(&functions, variant);
    finite_counts(&functions, variant);
}

fn zero_helpers(functions: &[&Value], variant: &str) {
    for name in ZERO_FUNCTIONS {
        let found = functions
            .iter()
            .filter(|function| function[NAME] == *name)
            .collect::<Vec<_>>();
        assert_eq!(
            found.len(),
            SINGLE,
            "{variant}: missing zero mapping {name}"
        );
        let function = found[0];
        assert_eq!(function[COUNT].as_u64(), Some(ZERO), "{variant}: {name}");
        let regions = function[REGIONS].as_array().unwrap();
        assert!(
            !regions.is_empty(),
            "{variant}: empty helper mappings {name}"
        );
        assert!(
            regions.iter().any(|region| {
                region[REGION_KIND_INDEX].as_u64() == Some(CODE_REGION_KIND)
                    && source_regions::fixture_file(function, region, REGION_FILE_ID_INDEX)
            }),
            "{variant}: helper {name} has no original fixture CodeRegion"
        );
        for region in regions {
            assert_eq!(
                region[COUNT_INDEX].as_u64(),
                Some(ZERO),
                "{variant}: {name}"
            );
        }
    }
}

fn caller_mappings(functions: &[&Value], variant: &str) {
    let mut entered_functions: Vec<&Value> = Vec::new();
    for marker in [ASYNC_ENTRY, CALLER_ENTRY, DROP_ENTRY, MAIN_ENTRY] {
        let entry = source_regions::code(functions, marker, variant);
        assert!(
            entered_functions
                .iter()
                .all(|previous| !std::ptr::eq(*previous, entry.function)),
            "{variant}: {marker} reused another caller's function mapping"
        );
        entered_functions.push(entry.function);
        assert_eq!(
            entry.function[COUNT].as_u64(),
            Some(ONCE),
            "{variant}: entry {marker}"
        );
        assert_eq!(
            entry.region[COUNT_INDEX].as_u64(),
            Some(ONCE),
            "{variant}: body {marker}"
        );
    }
    for (entry_marker, continuation) in [(ASYNC_ENTRY, AFTER_AWAIT), (CALLER_ENTRY, AFTER_DROP)] {
        let entry = source_regions::code(functions, entry_marker, variant);
        let after = source_regions::code(functions, continuation, variant);
        assert!(
            std::ptr::eq(entry.function, after.function),
            "{variant}: continuation {continuation} is not mapped to its entered caller"
        );
        assert_eq!(
            after.region[COUNT_INDEX].as_u64(),
            Some(ZERO),
            "{variant}: unreached caller continuation {continuation}"
        );
    }
    let destructor = source_regions::code(functions, DROP_ENTRY, variant);
    let branch = source_regions::branch(functions, DROP_CONDITION, variant);
    assert!(
        std::ptr::eq(destructor.function, branch.function),
        "{variant}: destructor condition is not mapped to the entered Drop body"
    );
    assert_eq!(
        branch.region[COUNT_INDEX].as_u64(),
        Some(ONCE),
        "{variant}: Drop true branch"
    );
    assert_eq!(
        branch.region[FALSE_COUNT_INDEX].as_u64(),
        Some(ZERO),
        "{variant}: Drop false branch"
    );
}

fn finite_counts(functions: &[&Value], variant: &str) {
    for function in functions {
        assert!(
            function[COUNT].as_u64().unwrap() < SATURATED,
            "{variant}: {function}"
        );
        for region in function[REGIONS].as_array().unwrap() {
            assert!(
                region[COUNT_INDEX].as_u64().unwrap() < SATURATED,
                "{variant}: {function}"
            );
        }
        for branch in function[BRANCHES].as_array().unwrap() {
            assert!(
                branch[COUNT_INDEX].as_u64().unwrap() < SATURATED,
                "{variant}: {function}"
            );
            assert!(
                branch[FALSE_COUNT_INDEX].as_u64().unwrap() < SATURATED,
                "{variant}: {function}"
            );
        }
    }
}
