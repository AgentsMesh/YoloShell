use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_authority_sites::TEST_BRIDGES;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    inspect_definitions(file, source, violations);
    inspect_required_bridges(file, source, violations);
}

fn inspect_definitions(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for function in source.functions().filter(|function| bridge_like(function)) {
        let allowed = TEST_BRIDGES
            .iter()
            .any(|site| site.matches(&file.relative, Some(function)));
        if !allowed {
            violations.push(Violation::line(
                &file.relative,
                function.line,
                contract::BRIDGE_OWNER_MESSAGE,
            ));
        } else if !source.is_test_guarded(function) {
            violations.push(Violation::line(
                &file.relative,
                function.line,
                contract::BRIDGE_CFG_MESSAGE,
            ));
        }
    }
}

fn inspect_required_bridges(
    file: &SourceFile,
    source: &RustSource,
    violations: &mut Vec<Violation>,
) {
    for site in TEST_BRIDGES {
        if site.path != file.relative {
            continue;
        }
        let count = source
            .functions()
            .filter(|function| function.matches(site.owner, site.function))
            .count();
        if count != 1 {
            violations.push(Violation::file(
                &file.relative,
                contract::BRIDGE_OWNER_MESSAGE,
            ));
        }
    }
}

fn bridge_like(function: &RustFunction) -> bool {
    function.name == contract::SIGNAL_FOR_TEST
        || function.name == contract::AUTHORIZE_FOR_TEST
        || function.owner.as_deref() == Some(contract::PROCESS_SIGNAL_AUTHORITY)
            && function.name == contract::FOR_TEST
        || matches!(
            function.owner.as_deref(),
            Some(contract::LIVE_IDENTITY | contract::PROCESS_IDENTITY)
        ) && function.name == contract::SIGNAL
}
