use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_signal_authority_sites as authority_sites;
use yolo_architecture_process_signal_foundation::process_signal_retained_bindings::Context as RetainedContext;
use yolo_architecture_process_signal_foundation::process_signal_sites as sites;
use yolo_architecture_process_signal_foundation::process_signal_syntax as syntax;
use yolo_architecture_process_signal_foundation::process_signal_type_paths::TYPE_PATHS;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const PROCESS_TREE_PREFIX: &str = "core/crates/runtime/engine/process/src/";

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    inspect_type_ownership(file, source, violations);
    inspect_authorized_entry(file, source, violations);
    inspect_constructors(file, source, violations);
    yolo_architecture_process_signal_foundation::process_signal_retained_operations::inspect(
        file, source, violations,
    );
    let retained =
        yolo_architecture_process_signal_path_analysis::types::type_names(&source.tokens);
    yolo_architecture_process_signal_path_analysis::deref::inspect(
        file, source, &retained, violations,
    );
    inspect_authority_signal_calls(file, source, violations);
    inspect_required_functions(file, source, violations);
}

fn inspect_type_ownership(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if is_rust_test_source(&file.relative) {
        return;
    }
    for token in &source.tokens {
        let Some(name) = identifier(token) else {
            continue;
        };
        let Some((_, paths)) = TYPE_PATHS.iter().find(|(sensitive, _)| *sensitive == name) else {
            continue;
        };
        if !paths.contains(&file.relative.as_str()) {
            violation(file, token, contract::AUTHORITY_TYPE_MESSAGE, violations);
        }
    }
}

fn inspect_authorized_entry(
    file: &SourceFile,
    source: &RustSource,
    violations: &mut Vec<Violation>,
) {
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(contract::SIGNAL_AUTHORIZED) {
            continue;
        }
        let function = source.function_at(index);
        let allowed = if syntax::definition(&source.tokens, index) {
            function.is_some_and(|function| {
                file.relative == contract::LIVE_CONTROL_PATH
                    && function.matches(Some(contract::LIVE_IDENTITY), contract::SIGNAL_AUTHORIZED)
            })
        } else {
            syntax::call(&source.tokens, index)
                && sites::AUTHORIZED_CALL_SITES
                    .iter()
                    .any(|site| site.matches(&file.relative, function))
        };
        if !allowed {
            violation(file, token, contract::AUTHORIZED_ENTRY_MESSAGE, violations);
        }
    }
}

fn inspect_constructors(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    let retained = RetainedContext::new(source);
    for (index, token) in source.tokens.iter().enumerate() {
        let Some((raw_qualifier, raw_name)) = syntax::qualified_reference(&source.tokens, index)
            .or_else(|| syntax::tuple_constructor(&source.tokens, index).map(|name| (name, name)))
        else {
            continue;
        };
        let function = source.function_at(index);
        let qualifier = retained
            .canonical_name(raw_qualifier)
            .unwrap_or_else(|| resolve_self(raw_qualifier, function));
        let name = resolve_self(raw_name, function);
        if !syntax::sensitive_constructor(qualifier, name) {
            continue;
        }
        let allowed = authority_sites::CONSTRUCTOR_SITES
            .iter()
            .chain(authority_sites::MEMBER_TEST_CONSTRUCTORS)
            .chain(yolo_architecture_process_signal_foundation::process_signal_lifecycle_constructor_sites::SITES)
            .any(|entry| {
                entry.qualifier == qualifier
                    && entry.name == name
                    && entry.site.matches(&file.relative, function)
            });
        if !allowed {
            violation(
                file,
                token,
                contract::AUTHORITY_CONSTRUCTOR_MESSAGE,
                violations,
            );
        }
    }
}

fn inspect_authority_signal_calls(
    file: &SourceFile,
    source: &RustSource,
    violations: &mut Vec<Violation>,
) {
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(contract::SIGNAL) {
            continue;
        }
        let function = source.function_at(index);
        let method = syntax::method_call(&source.tokens, index)
            && authority_receiver(file, &source.tokens, index);
        let associated = syntax::qualified_reference(&source.tokens, index)
            .is_some_and(|(qualifier, _)| sensitive_authority(resolve_self(qualifier, function)));
        if !method && !associated {
            continue;
        }
        if !authority_sites::AUTHORITY_SIGNAL_SITES
            .iter()
            .any(|site| site.matches(&file.relative, function))
        {
            violation(file, token, contract::AUTHORIZED_ENTRY_MESSAGE, violations);
        }
    }
}

fn resolve_self<'a>(
    name: &'a str,
    function: Option<&'a yolo_architecture_syntax::rust_source::RustFunction>,
) -> &'a str {
    if name == contract::SELF_TYPE {
        function
            .and_then(|function| function.owner.as_deref())
            .unwrap_or(name)
    } else {
        name
    }
}

fn sensitive_authority(name: &str) -> bool {
    matches!(
        name,
        contract::PROCESS_SIGNAL_AUTHORITY
            | contract::DESTRUCTIVE_AUTHORITY
            | contract::FOREGROUND_AUTHORITY
    )
}

fn authority_receiver(file: &SourceFile, tokens: &[Token], index: usize) -> bool {
    syntax::sensitive_receiver(tokens, index)
        || file.relative.starts_with(PROCESS_TREE_PREFIX)
            && !is_rust_test_source(&file.relative)
            && syntax::call_has_argument_separator(tokens, index)
}

fn inspect_required_functions(
    file: &SourceFile,
    source: &RustSource,
    violations: &mut Vec<Violation>,
) {
    for site in authority_sites::REQUIRED_AUTHORITY_FUNCTIONS {
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
                contract::AUTHORITY_CONSTRUCTOR_MESSAGE,
            ));
        }
    }
}

fn violation(file: &SourceFile, token: &Token, message: &str, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, message));
}
