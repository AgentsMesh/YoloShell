use super::*;
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

#[test]
fn permit_drop_requires_a_complete_standalone_statement() {
    let tokens = tokenize(&format!("{{ drop({PERMIT_BINDING}); }}"));
    let drop = 1;
    assert!(exact_drop(&tokens, drop));
    for index in 0..drop + 5 {
        for kind in [
            Kind::Identifier("other".into()),
            Kind::Symbol("other".into()),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!exact_drop(&changed, drop), "{index}");
        }
    }
    for length in 0..drop + 5 {
        assert!(!exact_drop(&tokens[..length], drop));
    }
    assert!(!exact_drop(
        &tokenize(&format!("drop({PERMIT_BINDING});")),
        0
    ));
    assert!(exact_drop(
        &tokenize(&format!("; drop({PERMIT_BINDING});")),
        drop
    ));
}

#[test]
fn submission_drop_and_await_order_must_hold_in_every_branch() {
    for (body, expected) in [
        (
            format!("{SUBMIT_TERMINATION}(); drop({PERMIT_BINDING}); work.await;"),
            true,
        ),
        (
            format!("drop({PERMIT_BINDING}); {SUBMIT_TERMINATION}(); work.await;"),
            false,
        ),
        (
            format!("{SUBMIT_TERMINATION}(); work.await; drop({PERMIT_BINDING});"),
            false,
        ),
        (
            format!("{SUBMIT_TERMINATION}(); drop({PERMIT_BINDING});"),
            false,
        ),
        (
            format!("{SUBMIT_TERMINATION}(); {{ drop({PERMIT_BINDING}); }} work.await;"),
            false,
        ),
        (
            format!("{SUBMIT_TERMINATION}(); drop({PERMIT_BINDING}); {{ work.await; }}"),
            false,
        ),
        (
            format!("{{ {SUBMIT_TERMINATION}(); }} drop({PERMIT_BINDING}); work.await;"),
            false,
        ),
        (
            format!("{SUBMIT_TERMINATION}(); {{ drop({PERMIT_BINDING}); work.await; }}"),
            false,
        ),
        (
            format!(
                "{SUBMIT_TERMINATION}(); {SUBMIT_TERMINATION}(); drop({PERMIT_BINDING}); work.await;"
            ),
            false,
        ),
    ] {
        let text = format!("P: {TERMINATION_PERMIT}, {PERMIT_BINDING}: P {{ {body} }}");
        assert_eq!(valid_termination(&tokenize(&text)), expected, "{body}");
    }
    assert!(!valid_termination(&tokenize(
        "{ submit(); drop(permit); work.await; }"
    )));
}

#[test]
fn permit_type_sequences_reject_each_identifier_and_separator_mutation() {
    let tokens = tokenize(&format!("P: {TERMINATION_PERMIT}, {PERMIT_BINDING}: P"));
    assert!(typed_permit(&tokens));
    for index in [0, 1, 2, 4, 5, 6] {
        let mut changed = tokens.clone();
        changed[index].kind = Kind::Number(0);
        assert!(!typed_permit(&changed), "{index}");
    }
}
