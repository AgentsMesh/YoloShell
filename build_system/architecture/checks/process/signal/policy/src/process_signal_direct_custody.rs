use crate::process_signal_direct_custody_sites::DIRECT_TERMINATION_SITES;
use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_unidentified_custody as unidentified;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::Token;

const MEMBER_ACCESS: &str = ".";
const CALL_OPEN: &str = "(";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if is_rust_test_source(&file.relative) {
        return;
    }
    let mut accepted = vec![false; DIRECT_TERMINATION_SITES.len()];
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(contract::REQUEST_DIRECT_TERMINATION)
            || unit_test_function(source, index)
        {
            continue;
        }
        if definition(file, source, index) {
            continue;
        }
        let Some(site) = DIRECT_TERMINATION_SITES
            .iter()
            .position(|site| call_matches(file, source, index, site))
        else {
            violation(file, token, violations);
            continue;
        };
        if accepted[site] {
            violation(file, token, violations);
        }
        accepted[site] = true;
    }
    for (index, site) in DIRECT_TERMINATION_SITES.iter().enumerate() {
        if site.site.path == file.relative && !accepted[index] {
            violations.push(Violation::file(
                &file.relative,
                contract::DIRECT_CUSTODY_MESSAGE,
            ));
        }
    }
}

fn definition(file: &SourceFile, source: &RustSource, index: usize) -> bool {
    let owner = match file.relative.as_str() {
        contract::DETACHED_CHILD_CUSTODY_PATH => contract::DETACHED_CHILD_CUSTODY,
        unidentified::PATH => unidentified::OWNER,
        _ => return false,
    };
    source
        .function_at(index)
        .is_some_and(|function| function.matches(Some(owner), contract::REQUEST_DIRECT_TERMINATION))
}

fn call_matches(
    file: &SourceFile,
    source: &RustSource,
    index: usize,
    site: &yolo_architecture_process_signal_foundation::process_signal_sites::ReceiverSite,
) -> bool {
    site.site.matches(&file.relative, source.function_at(index))
        && matches!(
            source.tokens.get(index.wrapping_sub(2)..index + 2),
            Some([receiver, member, _, open])
                if receiver.identifier(site.receiver)
                    && member.symbol(MEMBER_ACCESS)
                    && open.symbol(CALL_OPEN)
        )
}

fn unit_test_function(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_unit_test_guarded(function))
}

fn violation(file: &SourceFile, token: &Token, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(
        &file.relative,
        token.line,
        contract::DIRECT_CUSTODY_MESSAGE,
    ));
}

#[cfg(test)]
#[path = "../unit_tests/direct_custody_shape_tests.rs"]
mod shape_tests;
