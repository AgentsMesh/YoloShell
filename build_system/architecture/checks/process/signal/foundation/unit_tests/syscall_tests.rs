use yolo_architecture_policy_contracts::process_signal as signal;
use yolo_architecture_process_signal_foundation::process_signal_syscall_syntax as syntax;
use yolo_architecture_syntax::targeted_tokens::tokenize;

#[test]
fn syscall_names_and_nested_arguments_require_exact_libc_paths() {
    for (expression, expected) in [
        (
            "libc::syscall(libc::SYS_kill, nested(first, second), signal)",
            Some(signal::KILL_SYSCALL),
        ),
        ("libc::syscall(libc::SYS_kill,)", Some(signal::KILL_SYSCALL)),
        ("libc::syscall(other::SYS_kill, value)", None),
        ("libc::syscall(SYS_kill, value)", None),
        ("libc::syscall()", None),
        ("libc::syscall", None),
        ("libc::syscall(unclosed", None),
    ] {
        let tokens = tokenize(expression);
        let call = tokens
            .iter()
            .position(|token| token.identifier("syscall"))
            .unwrap();
        assert_eq!(syntax::call_name(&tokens, call), expected, "{expression}");
    }
    let tokens = tokenize("libc::syscall(libc::SYS_kill, nested(first, second), signal)");
    let call = tokens
        .iter()
        .position(|token| token.identifier("syscall"))
        .unwrap();
    assert_eq!(syntax::arguments(&tokens, call).unwrap().len(), 3);
    let nested = tokens
        .iter()
        .position(|token| token.identifier("second"))
        .unwrap();
    assert_eq!(syntax::containing_call(&tokens, nested), Some(call));
    assert_eq!(syntax::containing_call(&tokens, 0), None);
}

#[test]
fn libc_references_are_not_confused_with_methods_or_macro_arguments() {
    for (expression, expected) in [
        ("kill()", false),
        ("other::kill()", false),
        ("libc.kill()", false),
        ("libc::kill()", true),
    ] {
        let tokens = tokenize(expression);
        let index = tokens
            .iter()
            .position(|token| token.identifier("kill"))
            .unwrap();
        assert_eq!(syntax::libc_call(&tokens, index), expected);
    }
    for (expression, expected) in [
        ("record!(libc::kill())", true),
        ("record!{libc::kill()}", true),
        ("record![libc::kill()]", true),
        ("record(libc::kill())", false),
    ] {
        let tokens = tokenize(expression);
        let index = tokens
            .iter()
            .position(|token| token.identifier("kill"))
            .unwrap();
        assert_eq!(syntax::macro_argument(&tokens, index), expected);
    }
}
