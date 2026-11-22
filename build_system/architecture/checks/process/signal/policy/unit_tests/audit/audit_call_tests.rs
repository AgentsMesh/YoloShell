use super::fixture::*;
use crate::process_signal_audit_provenance as provenance;
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn wrapper_and_static_usage_require_the_exact_launch_bound_final_argument() {
    let capability = format!(
        "*{}.{}({})",
        contract::AUDIT_STATIC_BINDING,
        contract::GET_OR_INIT,
        contract::AUDIT_RESOLVER_FUNCTION
    );
    for (arguments, approved) in [
        (format!("identity, signal, {capability}"), true),
        (format!("identity, signal, {capability},"), true),
        (capability.clone(), false),
        (format!("identity, signal, {capability}, other"), false),
        (format!("identity, signal, &{}", &capability[1..]), false),
        (
            format!(
                "identity, signal, {}",
                capability.replace(contract::GET_OR_INIT, "other")
            ),
            false,
        ),
        (
            format!(
                "identity, signal, {}",
                capability.replace(contract::AUDIT_RESOLVER_FUNCTION, "other")
            ),
            false,
        ),
    ] {
        let text = format!(
            "fn {}() {{ {}({arguments}); }}",
            contract::AUDIT_ENTRY_FUNCTION,
            contract::AUDIT_WRAPPER_FUNCTION
        );
        assert_eq!(
            check(
                contract::MACOS_SIGNAL_PATH,
                &text,
                contract::AUDIT_WRAPPER_FUNCTION,
                provenance::approved_wrapper_reference
            ),
            approved,
            "{text}"
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
            contract::AUDIT_WRAPPER_FUNCTION,
            provenance::approved_wrapper_reference
        ));
    }
}

#[test]
fn wrapper_references_do_not_escape_approved_functions_except_in_explicit_tests() {
    let reference = format!("fn other() {{ {}; }}", contract::AUDIT_WRAPPER_FUNCTION);
    assert!(!check(
        contract::MACOS_SIGNAL_PATH,
        &reference,
        contract::AUDIT_WRAPPER_FUNCTION,
        provenance::approved_wrapper_reference
    ));
    assert!(check(
        TEST_PATH,
        &reference,
        contract::AUDIT_WRAPPER_FUNCTION,
        provenance::approved_wrapper_reference
    ));
    assert!(check(
        OTHER_PATH,
        &format!("#[cfg(test)] {reference}"),
        contract::AUDIT_WRAPPER_FUNCTION,
        provenance::approved_wrapper_reference
    ));
    let definition = format!("fn {}() {{}}", contract::AUDIT_WRAPPER_FUNCTION);
    assert!(check(
        contract::MACOS_SIGNAL_PATH,
        &definition,
        contract::AUDIT_WRAPPER_FUNCTION,
        provenance::approved_wrapper_reference
    ));
    assert!(!check(
        OTHER_PATH,
        &definition,
        contract::AUDIT_WRAPPER_FUNCTION,
        provenance::approved_wrapper_reference
    ));
    let incomplete = format!(
        "fn {}() {{ {}( {} ; }}",
        contract::AUDIT_ENTRY_FUNCTION,
        contract::AUDIT_WRAPPER_FUNCTION,
        contract::AUDIT_STATIC_BINDING
    );
    assert!(!check(
        contract::MACOS_SIGNAL_PATH,
        &incomplete,
        contract::AUDIT_STATIC_BINDING,
        provenance::approved_static_binding
    ));
}
