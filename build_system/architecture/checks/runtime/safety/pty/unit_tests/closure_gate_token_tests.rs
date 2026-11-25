use super::*;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn gate_success_header_requires_every_token_of_the_guard_and_lock_operation() {
    let tokens = tokenize(&format!("if let Ok(permit) = gate.{TRY_LOCK}() {{"));
    let open = tokens.len() - 1;
    assert!(successful_try_lock_header(&tokens, open));
    for index in (0..7).chain(open - 3..open) {
        for kind in [
            Kind::Identifier("other".into()),
            Kind::Symbol("other".into()),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!successful_try_lock_header(&changed, open), "{index}");
        }
    }
    for text in [
        "{",
        "if {",
        "if let Ok(permit) = other {",
        "if let Ok(permit) = lock(other) {",
    ] {
        let tokens = tokenize(text);
        assert!(
            !successful_try_lock_header(&tokens, tokens.len() - 1),
            "{text}"
        );
    }
}

#[test]
fn permit_consumption_detector_requires_the_complete_drop_expression() {
    let tokens = tokenize("{ drop(permit); }");
    assert!(permit_dropped_before(&tokens, 0, tokens.len()));
    for index in 1..5 {
        let mut changed = tokens.clone();
        changed[index].kind = Kind::Number(0);
        assert!(
            !permit_dropped_before(&changed, 0, changed.len()),
            "{index}"
        );
    }
    assert!(!permit_dropped_before(&tokens, 0, 4));
    assert!(identifier(&tokenize("5")[0]).is_none());
}

#[test]
fn typed_task_reference_cannot_be_supplied_only_by_the_body_or_mutable_reference() {
    for (parameter, body, expected) in [
        (format!("_: &{TASK_PERMIT}"), "", true),
        (format!("_: {TASK_PERMIT}"), "", false),
        (format!("_: &mut {TASK_PERMIT}"), "", false),
        ("_: &Other".into(), "", false),
        ("".into(), "let candidate: &Other;", false),
    ] {
        let source = RustSource::parse(&format!("fn task({parameter}) {{ {body} }}"));
        let function = source.functions().next().unwrap();
        assert_eq!(
            typed_reference_parameter(&source, function, TASK_PERMIT),
            expected
        );
    }
    let source = RustSource::parse(&format!("fn task() {{ let candidate: &{TASK_PERMIT}; }}"));
    assert!(!typed_reference_parameter(
        &source,
        source.functions().next().unwrap(),
        TASK_PERMIT
    ));
}
