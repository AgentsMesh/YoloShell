use super::support::{assert_flow, reject_mutations, source};
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn bridge_requires_one_custodied_child_and_exact_rooted_process_calls() {
    reject_mutations(
        contract::BRIDGE_CHILD_PATH,
        &[
            (
                "let Some(mut child) = self.child.take()",
                "let Some(mut child) = self.other.take()",
            ),
            (
                "let observed = ::std::process::Child::try_wait(&mut child);",
                "let observed = Ok(None);",
            ),
            (
                "let observed = ::std::process::Child::try_wait(&mut child);",
                "let observed = ::std::process::Child::try_wait(&mut child); child = replacement();",
            ),
            (
                "let observed = ::std::process::Child::try_wait(&mut child);",
                "let observed = ::std::process::Child::try_wait(&mut child); let mut child = replacement();",
            ),
            ("match observed {", "match unrelated {"),
            (
                "::std::process::Child::kill(&mut child)",
                "::std::process::Child::kill(&mut other)",
            ),
            (
                "::std::process::Child::wait(&mut child)",
                "::std::process::Child::wait(&mut other)",
            ),
            (
                "let terminated = ::std::process::Child::kill(&mut child);",
                "let terminated = Ok(());",
            ),
        ],
    );
}

#[test]
fn bridge_settlement_cannot_be_shadowed_wrapped_or_cfg_disabled() {
    reject_mutations(
        contract::BRIDGE_CHILD_PATH,
        &[
            (
                "match observed {",
                "let settle_failed_child = unrelated; match observed {",
            ),
            (
                "match observed {",
                "fn settle_failed_child() {} match observed {",
            ),
            (
                "settle_failed_child(child, error,",
                "settle_failed_child!(child, error,",
            ),
            ("#[cfg(test)]", "#[cfg(feature = \"conditional-cleanup\")]"),
            (
                "let terminated = ::std::process::Child::kill(&mut child);",
                "let terminated = { ::std::process::Child::kill(&mut child) };",
            ),
            (
                "if let Err(error) = ::std::process::Child::wait(&mut child)",
                "if let Ok(error) = ::std::process::Child::wait(&mut child)",
            ),
        ],
    );
}

#[test]
fn bridge_rejects_duplicate_kill_wait_poll_and_destructor_definitions() {
    let baseline = source(contract::BRIDGE_CHILD_PATH);
    for name in [
        contract::KILL_METHOD,
        contract::WAIT_METHOD,
        contract::TRY_WAIT,
    ] {
        let statement = format!("::std::process::Child::{name}(&mut child);");
        let changed = baseline.replace(
            "match observed {",
            &format!("{statement} match observed {{"),
        );
        assert_flow(contract::BRIDGE_CHILD_PATH, &changed, false);
    }
    assert_flow(
        contract::BRIDGE_CHILD_PATH,
        &format!("{baseline}\n{baseline}"),
        false,
    );
    assert_flow(contract::BRIDGE_CHILD_PATH, "", false);
}

#[test]
fn rooted_child_flow_requires_each_argument_token_and_cannot_truncate_it() {
    use crate::process_signal_child_flow_bridge_dataflow::exact_rooted_child_call;
    use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};
    const REPLACEMENT: &str = "untrusted";
    let tokens = tokenize("::std::process::Child::try_wait(&mut child)");
    let call = tokens
        .iter()
        .position(|token| token.identifier(contract::TRY_WAIT))
        .unwrap();
    assert!(exact_rooted_child_call(&tokens, call, contract::TRY_WAIT));
    for index in 0..tokens.len() {
        for kind in [
            Kind::Identifier(REPLACEMENT.into()),
            Kind::Symbol(REPLACEMENT.into()),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(
                !exact_rooted_child_call(&changed, call, contract::TRY_WAIT),
                "{index}"
            );
        }
    }
    for length in call + 1..tokens.len() {
        assert!(!exact_rooted_child_call(
            &tokens[..length],
            call,
            contract::TRY_WAIT
        ));
    }
}
