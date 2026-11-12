use super::fixture::*;
use crate::process_signal_audit_provenance as provenance;
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn symbol_resolution_is_bound_to_the_exact_file_and_free_resolver() {
    for (text, approved) in [
        (
            format!("const {}: usize = {PROBE};", contract::AUDIT_SYMBOL_BINDING),
            true,
        ),
        (
            format!(
                "fn {}() {{ {PROBE}(); }}",
                contract::AUDIT_RESOLVER_FUNCTION
            ),
            true,
        ),
        (format!("const other: usize = {PROBE};"), false),
        (format!("fn other() {{ {PROBE}(); }}"), false),
        (
            format!(
                "impl Owner {{ fn {}() {{ {PROBE}(); }} }}",
                contract::AUDIT_RESOLVER_FUNCTION
            ),
            false,
        ),
        (PROBE.to_owned(), false),
    ] {
        assert_eq!(
            check(
                contract::MACOS_SIGNAL_PATH,
                &text,
                PROBE,
                provenance::approved_symbol
            ),
            approved,
            "{text}"
        );
        assert!(!check(
            OTHER_PATH,
            &text,
            PROBE,
            provenance::approved_symbol
        ));
    }
}

#[test]
fn audit_function_type_has_only_its_declaration_resolver_and_wrapper_contexts() {
    for (text, approved) in [
        (format!("type {PROBE} = usize;"), true),
        (
            format!(
                "static {}: Option<{PROBE}> = None;",
                contract::AUDIT_STATIC_BINDING
            ),
            true,
        ),
        (
            format!("fn {}() {{ {PROBE}; }}", contract::AUDIT_WRAPPER_FUNCTION),
            true,
        ),
        (
            format!("fn {}() {{ {PROBE}; }}", contract::AUDIT_RESOLVER_FUNCTION),
            true,
        ),
        (format!("const OTHER: usize = {PROBE};"), false),
        (format!("fn other() {{ {PROBE}; }}"), false),
        (PROBE.to_owned(), false),
    ] {
        assert_eq!(
            check(
                contract::MACOS_SIGNAL_PATH,
                &text,
                PROBE,
                provenance::approved_function_type
            ),
            approved,
            "{text}"
        );
        assert!(!check(
            OTHER_PATH,
            &text,
            PROBE,
            provenance::approved_function_type
        ));
    }
}

#[test]
fn static_binding_requires_its_static_declaration_or_exact_capability_probe() {
    for (tail, approved) in [
        (
            format!(
                ".get_or_init({}).is_some()",
                contract::AUDIT_RESOLVER_FUNCTION
            ),
            true,
        ),
        (
            format!(
                ".get_or_init({}).is_none()",
                contract::AUDIT_RESOLVER_FUNCTION
            ),
            false,
        ),
        (
            format!(
                "::get_or_init({}).is_some()",
                contract::AUDIT_RESOLVER_FUNCTION
            ),
            false,
        ),
        (".get_or_init(other).is_some()".to_owned(), false),
        ("".to_owned(), false),
    ] {
        let text = format!(
            "fn {}() {{ {}{tail}; }}",
            contract::AUDIT_CAPABILITY_FUNCTION,
            contract::AUDIT_STATIC_BINDING
        );
        assert_eq!(
            check(
                contract::MACOS_SIGNAL_PATH,
                &text,
                contract::AUDIT_STATIC_BINDING,
                provenance::approved_static_binding
            ),
            approved,
            "{text}"
        );
        assert!(!check(
            OTHER_PATH,
            &text,
            contract::AUDIT_STATIC_BINDING,
            provenance::approved_static_binding
        ));
    }
    assert!(check(
        contract::MACOS_SIGNAL_PATH,
        &format!("static {PROBE}: usize = 0;"),
        PROBE,
        provenance::approved_static_binding
    ));
    assert!(!check(
        contract::MACOS_SIGNAL_PATH,
        &format!("const {PROBE}: usize = 0;"),
        PROBE,
        provenance::approved_static_binding
    ));
}
