use super::constants::*;
use crate::process_signal_child_flow as flow;
use yolo_architecture_syntax::rust_source::RustSource;

#[test]
fn unique_function_requires_one_definition_with_the_exact_owner() {
    let source = RustSource::parse(&format!("impl {OWNER} {{ fn {FIRST_FUNCTION}() {{}} }}"));
    assert!(flow::unique_function(&source, Some(OWNER), FIRST_FUNCTION).is_some());
    assert!(flow::unique_function(&source, None, FIRST_FUNCTION).is_none());
    assert!(flow::unique_function(&source, Some(OWNER), SECOND_FUNCTION).is_none());
    let duplicate = RustSource::parse(&format!(
        "impl {OWNER} {{ #[cfg(test)] fn {FIRST_FUNCTION}() {{}} #[cfg(not(test))] fn {FIRST_FUNCTION}() {{}} }}"
    ));
    assert!(flow::unique_function(&duplicate, Some(OWNER), FIRST_FUNCTION).is_none());
}

#[test]
fn body_cfg_allows_only_test_guarded_conditional_cleanup() {
    for (attribute, rejected) in [
        ("", false),
        ("#[allow(unused)]", false),
        ("#[cfg(test)]", false),
        ("#[cfg(all(test, unix))]", false),
        ("#[cfg(not(test))]", true),
        ("#[cfg(any(test, unix))]", true),
        ("#[cfg(test)", true),
    ] {
        let source = RustSource::parse(&format!(
            "fn {FIRST_FUNCTION}() {{ {attribute} action(); }}"
        ));
        let function = flow::unique_function(&source, None, FIRST_FUNCTION).unwrap();
        assert_eq!(
            flow::production_cfg_in_body(&source.tokens, function),
            rejected,
            "{attribute}"
        );
    }
}

#[test]
fn macro_arguments_and_function_references_are_not_execution_evidence() {
    let source = RustSource::parse(&format!(
        "fn {FIRST_FUNCTION}() {{ capture!({SECOND_FUNCTION}()); {SECOND_FUNCTION}::reference; {SECOND_FUNCTION}(); }}"
    ));
    let function = flow::unique_function(&source, None, FIRST_FUNCTION).unwrap();
    assert_eq!(
        flow::calls(&source.tokens, function, SECOND_FUNCTION).len(),
        1
    );
    assert!(flow::one_call(&source, function, SECOND_FUNCTION).is_some());
    assert!(flow::one_call(&source, function, REPLACED_FUNCTION).is_none());
}

#[test]
fn ordering_and_assignment_evidence_is_unique_and_function_scoped() {
    for (body, ordered) in [
        (format!("{FIRST_FUNCTION}(); {SECOND_FUNCTION}();"), true),
        (format!("{SECOND_FUNCTION}(); {FIRST_FUNCTION}();"), false),
        (format!("{FIRST_FUNCTION}();"), false),
        (
            format!("{FIRST_FUNCTION}(); {FIRST_FUNCTION}(); {SECOND_FUNCTION}();"),
            false,
        ),
    ] {
        let source = RustSource::parse(&format!("fn operation() {{ {body} }}"));
        let function = source.functions().next().unwrap();
        assert_eq!(
            flow::ordered_once(&source, function, FIRST_FUNCTION, SECOND_FUNCTION),
            ordered
        );
        assert_eq!(
            flow::sequence_before(&source, function, &[FIRST_FUNCTION], &[SECOND_FUNCTION]),
            ordered
        );
    }
    let source = RustSource::parse(&format!(
        "fn operation() {{ let {BINDING} = false; }} fn other() {{ let {BINDING} = false; }}"
    ));
    let function = source.functions().next().unwrap();
    assert_eq!(
        flow::assignment_count(&source, function, BINDING, "false"),
        1
    );
    assert!(flow::contains(&source, function, &[BINDING, "=", "false"]));
    assert!(!flow::contains(&source, function, &[BINDING, "=", "true"]));
}
