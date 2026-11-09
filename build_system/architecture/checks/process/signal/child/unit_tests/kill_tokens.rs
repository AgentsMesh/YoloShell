use super::*;
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

const REPLACEMENT: &str = "untrusted";
const MUTATION_NUMBER: u64 = 7;
const ROOTED_PREFIX: &str = "::std::process::Child::kill";

#[test]
fn every_std_child_root_and_receiver_token_is_required() {
    for (argument, expected) in [
        ("self", contract::SELF_RECEIVER),
        ("&mut self.child", contract::SELF_CHILD_RECEIVER),
        ("&mut child", contract::CHILD_RECEIVER),
    ] {
        let tokens = tokenize(&format!("{ROOTED_PREFIX}({argument})"));
        let call = tokens
            .iter()
            .position(|token| token.identifier(contract::KILL_METHOD))
            .unwrap();
        assert!(exact_std_child_call(&tokens, call, expected));
        let mutations = (0..call).chain(call + 2..tokens.len() - 1);
        for index in mutations {
            for kind in [
                Kind::Identifier(REPLACEMENT.into()),
                Kind::Symbol(REPLACEMENT.into()),
                Kind::Number(MUTATION_NUMBER),
            ] {
                let mut changed = tokens.clone();
                changed[index].kind = kind;
                assert!(
                    !exact_std_child_call(&changed, call, expected),
                    "{argument}, {index}"
                );
            }
        }
        for length in 0..tokens.len() {
            assert!(
                !exact_std_child_call(&tokens[..length], call, expected),
                "{argument}, {length}"
            );
        }
    }
    assert!(!exact_std_child_call(&[], 0, contract::SELF_RECEIVER));
    let tokens = tokenize("::std::process::Child::kill(other)");
    let call = tokens
        .iter()
        .position(|token| token.identifier(contract::KILL_METHOD))
        .unwrap();
    assert!(!exact_std_child_call(
        &tokens,
        call,
        contract::SELF_RECEIVER
    ));
}

#[test]
fn portable_pty_kill_rejects_each_receiver_rebinding() {
    let tokens = tokenize("self.child.kill()");
    let call = tokens
        .iter()
        .position(|token| token.identifier(contract::KILL_METHOD))
        .unwrap();
    assert!(exact_portable_pty_child_call(&tokens, call));
    for index in 0..call {
        for kind in [
            Kind::Identifier(REPLACEMENT.into()),
            Kind::Symbol(REPLACEMENT.into()),
            Kind::Number(MUTATION_NUMBER),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!exact_portable_pty_child_call(&changed, call), "{index}");
        }
    }
    for length in 0..call {
        assert!(!exact_portable_pty_child_call(&tokens[..length], call));
    }
    assert!(!exact_portable_pty_child_call(&[], 0));
}
