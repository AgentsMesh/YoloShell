use super::*;
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

#[test]
fn broker_command_constructor_requires_exact_separator_and_constructor_identity() {
    let tokens = tokenize("Command::new");
    assert!(command_constructor(&tokens, 0));
    for index in 1..tokens.len() {
        for kind in [
            Kind::Identifier("other".into()),
            Kind::Symbol("other".into()),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!command_constructor(&changed, 0), "{index}");
        }
    }
    for length in 1..tokens.len() {
        assert!(!command_constructor(&tokens[..length], 0));
    }
}

#[test]
fn broker_clone_detection_is_not_confused_by_neighboring_uses_or_methods() {
    for (text, expected) in [
        ("clone", false),
        ("owner.clone()", false),
        ("other::clone()", false),
        ("libc.clone()", false),
        ("libc::clone()", true),
        ("fn clone() {}", true),
        ("use libc::{clone};", true),
        ("use other::{clone};", false),
        ("use libc::other; clone()", false),
        ("use other::x; use libc::{clone};", true),
        ("libc::other; use other::clone;", false),
    ] {
        let tokens = tokenize(text);
        let call = tokens
            .iter()
            .position(|token| token.identifier(contract::CLONE_IDENTIFIER))
            .unwrap();
        assert_eq!(raw_clone(&tokens, call), expected, "{text}");
    }
    let tokens = tokenize("std::process");
    for index in 0..tokens.len() {
        let mut changed = tokens.clone();
        changed[index].kind = Kind::Number(0);
        assert!(!contains_path(&changed, "std", contract::PROCESS_NAMESPACE));
    }
}
