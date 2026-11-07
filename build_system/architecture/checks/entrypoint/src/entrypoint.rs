use crate::constants::{
    self as contract, CRATE_ROOT_ATTRIBUTE, CORE_PREFIX, NAME_ATTRIBUTE, RUST_BINARY_RULE,
    RUST_LIBRARY_RULE, RUST_STATIC_LIBRARY_RULE, RUST_TEST_RULE, SOURCES_ATTRIBUTE,
    TEST_ONLY_ATTRIBUTE, TRUE_LITERAL,
};
use crate::dependencies;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_target_ownership::{
    Document, Expression, Repository, RuleCall, is_build_file,
};

pub fn inspect(repository: &Repository, violations: &mut Vec<Violation>) {
    let mut production_count = 0;
    for document in repository.documents().filter(|document| {
        document.path.starts_with(CORE_PREFIX) && is_build_file(&document.path)
    }) {
        inspect_calls(repository, document, violations);
        if document.package == contract::LAUNCH_PACKAGE {
            for rule in document
                .rules
                .iter()
                .filter(|rule| rule.name == RUST_LIBRARY_RULE)
            {
                dependencies::inspect_launch_contract(document, rule, violations);
            }
        }
        if document.rules_unresolved {
            violations.push(Violation::file(
                &document.path,
                contract::RULE_RESOLUTION_POLICY,
            ));
        }
        for rule in document
            .rules
            .iter()
            .filter(|rule| rule.name == RUST_BINARY_RULE)
        {
            if matches!(rule.attributes.get(TEST_ONLY_ATTRIBUTE),
                Some(Expression::Name(value)) if value == TRUE_LITERAL)
            {
                continue;
            }
            production_count += 1;
            if rule.attributes.get(TEST_ONLY_ATTRIBUTE).is_some_and(|value| {
                !matches!(value, Expression::Name(value) if value == contract::FALSE_LITERAL)
            }) {
                violations.push(Violation::line(&document.path, rule.line, contract::EXECUTABLE_POLICY));
            }
            let expected = document.package == contract::PACKAGE
                && matches!(rule.attributes.get(NAME_ATTRIBUTE),
                    Some(Expression::String(name)) if name == contract::NAME);
            if !expected {
                violations.push(Violation::line(
                    &document.path,
                    rule.line,
                    contract::EXECUTABLE_POLICY,
                ));
                continue;
            }
            composition(repository, document, rule, violations);
            dependencies::inspect(document, rule, violations);
        }
    }
    if production_count != contract::REQUIRED_EXECUTABLES {
        violations.push(Violation::file(
            contract::BUILD,
            format!("{}: found {production_count}", contract::EXECUTABLE_POLICY),
        ));
    }
}

fn inspect_calls(repository: &Repository, document: &Document, violations: &mut Vec<Violation>) {
    for call in &document.calls {
        let imported = document.imports.get(&call.name).map(|origin| {
            origin
                .split_once('#')
                .unwrap_or((origin.as_str(), call.name.as_str()))
        });
        let rust_rule = matches!(
            call.canonical.as_str(),
            RUST_BINARY_RULE | RUST_LIBRARY_RULE | RUST_STATIC_LIBRARY_RULE | RUST_TEST_RULE
        );
        let builtin = contract::SAFE_BUILTINS.contains(&call.name.as_str()) && imported.is_none();
        let reviewed = imported.is_some_and(|origin| contract::REVIEWED_MACROS.contains(&origin));
        let depset_conversion = call.qualified && call.depset_receiver && call.name == "to_list";
        if call.qualified && !depset_conversion
            || document.assignments.contains_key(&call.name)
            || !(builtin || reviewed || rust_rule && call.top_level || depset_conversion)
        {
            violations.push(Violation::line(
                &document.path,
                call.line,
                contract::CALLABLE_POLICY,
            ));
        }
        if let Some((source, _)) = imported.filter(|_| reviewed) {
            let Some(source) = repository.document(source) else {
                violations.push(Violation::line(
                    &document.path,
                    call.line,
                    contract::CALLABLE_POLICY,
                ));
                continue;
            };
            if !dependencies::macro_is_reviewed(
                repository,
                source,
                &mut std::collections::BTreeSet::new(),
            ) {
                violations.push(Violation::file(&source.path, contract::EXECUTABLE_POLICY));
            }
        }
    }
}

fn composition(
    repository: &Repository,
    document: &Document,
    rule: &RuleCall,
    violations: &mut Vec<Violation>,
) {
    let Some(sources) = rule.attributes.get(SOURCES_ATTRIBUTE) else {
        violations.push(Violation::line(
            &document.path,
            rule.line,
            contract::COMPOSITION_POLICY,
        ));
        return;
    };
    let evaluated = repository.evaluate(&document.path, &document.package, rule.line, sources);
    let root = format!("{}/{}", contract::PACKAGE, contract::ROOT);
    let local = evaluated
        .paths
        .iter()
        .all(|source| source.starts_with(contract::SOURCE_PREFIX));
    let lines: usize = evaluated
        .paths
        .iter()
        .map(|source| repository.rust_source_lines(source).unwrap_or_default())
        .sum();
    if !matches!(rule.attributes.get(CRATE_ROOT_ATTRIBUTE), Some(Expression::String(value)) if value == contract::ROOT)
        || evaluated.unresolved
        || !evaluated.labels.is_empty()
        || !local
        || !evaluated.paths.contains(&root)
        || evaluated.paths.len() > contract::MAX_SOURCES
        || lines > contract::MAX_LINES
    {
        violations.push(Violation::line(
            &document.path,
            rule.line,
            contract::COMPOSITION_POLICY,
        ));
    }
}
