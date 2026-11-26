use crate::process_signal_child_flow_syntax as syntax;
use yolo_architecture_policy_contracts::process_signal_retained::vocabulary::{
    KILL_METHOD as KILL, WAIT_METHOD as WAIT,
};
use yolo_architecture_syntax::targeted_tokens::tokenize;

const RANGE_START: usize = 0;

#[test]
fn successful_poll_arm_must_contain_both_process_operations() {
    let tokens = tokenize("Ok(None) => { kill(); wait(); }");
    let kill = tokens
        .iter()
        .position(|token| token.identifier(KILL))
        .unwrap();
    let wait = tokens
        .iter()
        .position(|token| token.identifier(WAIT))
        .unwrap();
    let close = tokens.len() - 1;
    assert!(syntax::match_arm_contains(
        &tokens,
        RANGE_START..tokens.len(),
        kill,
        wait
    ));
    assert!(!syntax::match_arm_contains(
        &tokens,
        RANGE_START..tokens.len(),
        RANGE_START,
        wait
    ));
    assert!(!syntax::match_arm_contains(
        &tokens,
        RANGE_START..tokens.len(),
        kill,
        close
    ));
    for source in [
        "Ok(None) => { kill(); wait();",
        "Ok(Some) => { kill(); wait(); }",
        "Err(None) => { kill(); wait(); }",
    ] {
        let tokens = tokenize(source);
        assert!(!syntax::match_arm_contains(
            &tokens,
            RANGE_START..tokens.len(),
            kill,
            wait
        ));
    }
}

#[test]
fn termination_error_branch_requires_top_level_return_and_exact_child_settlement() {
    for (source, expected) in [
        (
            "if let Err(error) = terminated { settle_failed_child(child, error); return; }",
            true,
        ),
        (
            "if let Err(error) = terminated { settle_failed_child(child, error); }",
            false,
        ),
        (
            "if let Err(error) = terminated { settle_failed_child(child, error); if condition { return; } }",
            false,
        ),
        ("if let Err(error) = terminated { return; }", false),
        (
            "if let Err(error) = terminated { settle_failed_child(other, error); return; }",
            false,
        ),
        (
            "if let Err(other) = terminated { settle_failed_child(child, error); return; }",
            false,
        ),
        (
            "if let Err(error) = terminated { settle_failed_child(child, error); return;",
            false,
        ),
    ] {
        let tokens = tokenize(source);
        assert_eq!(
            syntax::termination_error_returns(&tokens, RANGE_START, tokens.len()),
            expected,
            "{source}"
        );
        if expected {
            assert!(!syntax::termination_error_returns(
                &tokens,
                RANGE_START,
                tokens.len() - 1
            ));
        }
    }
}

#[test]
fn poll_errors_cannot_be_ignored_deferred_or_assigned_to_another_child() {
    for (source, expected) in [
        ("Err(error) => settle_failed_child(child, error)", true),
        ("Err(other) => settle_failed_child(child, other)", false),
        ("Err(error) => settle_failed_child(other, error)", false),
        ("Err(error) => { settle_failed_child(child, error) }", false),
        ("Err(error) => settle_failed_child!(child, error)", false),
        ("Err(error) => ignored(child, error)", false),
    ] {
        let tokens = tokenize(source);
        assert_eq!(
            syntax::poll_error_settles(&tokens, RANGE_START..tokens.len()),
            expected,
            "{source}"
        );
    }
}

#[test]
fn termination_result_must_be_bound_to_the_exact_local_before_the_rooted_call() {
    for (source, expected) in [
        (
            "let terminated = ::std::process::Child::kill(&mut child);",
            true,
        ),
        (
            "let ignored = ::std::process::Child::kill(&mut child);",
            false,
        ),
        ("kill(&mut child);", false),
    ] {
        let tokens = tokenize(source);
        let call = tokens
            .iter()
            .position(|token| token.identifier(KILL))
            .unwrap();
        assert_eq!(
            syntax::assignment_before(&tokens, call),
            expected,
            "{source}"
        );
    }
}
