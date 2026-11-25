use super::fixture::Fixture;
use super::process_signal_test_support::messages;
use yolo_architecture_policy_contracts::process_signal::{
    AUDIT_CALL_BINDING, AUDIT_SIGNAL_FUNCTION, AUDIT_WRAPPER_FUNCTION, MACOS_SIGNAL_PATH,
};

const AUDIT_POLICY: &str = "macOS audit-token signal primitive";
const RAW_POLICY: &str = "raw process signal primitive";
const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_audit.rs";

#[test]
fn accepts_the_exact_audit_token_wrapper_call() {
    let fixture = Fixture::new();
    fixture.write(
        MACOS_SIGNAL_PATH,
        &wrapper(AUDIT_CALL_BINDING, "function(&mut token, signal);"),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn renamed_parenthesized_or_spoofed_audit_calls_are_rejected() {
    for body in [
        "sender(&mut token, signal);",
        "(sender)(&mut token, signal);",
        "function(&mut token, signal); sender(&mut token, signal);",
    ] {
        let fixture = Fixture::new();
        fixture.write(MACOS_SIGNAL_PATH, &wrapper("sender", body));
        assert!(
            messages(&fixture)
                .iter()
                .any(|message| message.contains(AUDIT_POLICY))
        );
    }
}

#[test]
fn audit_function_pointer_aliases_cannot_move_the_launch_capability() {
    let renamed = Fixture::new();
    renamed.write(
        MACOS_SIGNAL_PATH,
        &format!(
            "{}\nfn bypass(sender: {AUDIT_SIGNAL_FUNCTION}) {{ unsafe {{ sender(&mut token, signal); }} }}\n",
            wrapper(AUDIT_CALL_BINDING, "function(&mut token, signal);")
        ),
    );
    assert!(contains(&messages(&renamed), AUDIT_POLICY));

    let aliased = Fixture::new();
    aliased.write(
        MACOS_SIGNAL_PATH,
        &wrapper(
            AUDIT_CALL_BINDING,
            "let sender = function; sender(&mut token, signal);",
        ),
    );
    assert!(contains(&messages(&aliased), AUDIT_POLICY));

    let resolver_alias = Fixture::new();
    resolver_alias.write(
        MACOS_SIGNAL_PATH,
        &format!(
            "{}\nfn resolve_signal_function() -> Option<{AUDIT_SIGNAL_FUNCTION}> {{ let sender: {AUDIT_SIGNAL_FUNCTION} = source(); unsafe {{ sender(&mut token, signal); }} }}\n",
            wrapper(AUDIT_CALL_BINDING, "function(&mut token, signal);")
        ),
    );
    assert!(contains(&messages(&resolver_alias), AUDIT_POLICY));
}

#[test]
fn wrapper_bindings_cannot_be_shadowed_or_rebound() {
    for source in [
        wrapper(
            AUDIT_CALL_BINDING,
            "let function = injected; function(&mut token, signal);",
        ),
        wrapper(
            AUDIT_CALL_BINDING,
            "function = injected; function(&mut token, signal);",
        ),
        wrapper(AUDIT_CALL_BINDING, "function(&mut token, signal);").replace(
            "let function = capability.context(missing)?;",
            "let capability = injected; let function = capability.context(missing)?;",
        ),
        wrapper(AUDIT_CALL_BINDING, "function(&mut token, signal);").replace(
            "capability.context(missing)?",
            "resolve_signal_function().context(missing)?",
        ),
        format!(
            "macro_rules! hijack {{ () => {{ let function = injected; }} }}\n{}",
            wrapper(
                AUDIT_CALL_BINDING,
                "hijack!(); function(&mut token, signal);"
            )
        ),
    ] {
        let fixture = Fixture::new();
        fixture.write(MACOS_SIGNAL_PATH, source);
        assert!(contains(&messages(&fixture), AUDIT_POLICY));
    }
}

#[test]
fn wrapper_input_must_come_from_the_launch_bound_static() {
    let injected = Fixture::new();
    injected.write(
        MACOS_SIGNAL_PATH,
        &format!(
            "{}\nfn signal_once() {{ signal_once_with_capability(identity, signal, injected); }}\n",
            wrapper(AUDIT_CALL_BINDING, "function(&mut token, signal);")
        ),
    );
    assert!(contains(&messages(&injected), AUDIT_POLICY));

    let moved = Fixture::new();
    moved.write(
        FUTURE_PATH,
        "fn bypass() { signal_once_with_capability(identity, signal, injected); }\n",
    );
    assert!(contains(&messages(&moved), AUDIT_POLICY));
}

#[test]
fn accepts_the_exact_static_to_wrapper_provenance() {
    let fixture = Fixture::new();
    fixture.write(
        MACOS_SIGNAL_PATH,
        &format!(
            "type {AUDIT_SIGNAL_FUNCTION} = unsafe extern \"C\" fn();\n\
             static SIGNAL_FUNCTION: OnceLock<Option<{AUDIT_SIGNAL_FUNCTION}>> = OnceLock::new();\n\
             fn resolve_signal_function() -> Option<{AUDIT_SIGNAL_FUNCTION}> {{ source() }}\n\
             fn signal_once() {{ signal_once_with_capability(identity, signal, *SIGNAL_FUNCTION.get_or_init(resolve_signal_function)); }}\n\
             fn {AUDIT_WRAPPER_FUNCTION}(capability: Option<{AUDIT_SIGNAL_FUNCTION}>) {{ let function = capability.context(missing)?; unsafe {{ function(&mut token, signal); }} }}\n"
        ),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn escaped_link_names_and_dynamic_symbols_are_still_signal_primitives() {
    for source in [
        r#"extern "C" { #[link_name = "ki\x6c\x6c"] fn hidden(pid: i32, signal: i32); }"#,
        r#"extern "C" { #[link_name = "\u{06b}\u{069}\u{06c}\u{06c}"] fn hidden(); }"#,
        r#"fn bypass() { libc::dlsym(handle, b"ki\x6c\x6c\0".as_ptr()); }"#,
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert!(contains(&messages(&fixture), RAW_POLICY), "{source}");
    }
    let audit = Fixture::new();
    audit.write(
        FUTURE_PATH,
        r#"const SYMBOL: &str = "proc_signal_with_auditto\x6ben\0";"#,
    );
    assert!(contains(&messages(&audit), AUDIT_POLICY));
}

fn wrapper(binding: &str, body: &str) -> String {
    format!(
        "type {AUDIT_SIGNAL_FUNCTION} = unsafe extern \"C\" fn();\n\
         fn {AUDIT_WRAPPER_FUNCTION}(capability: Option<{AUDIT_SIGNAL_FUNCTION}>) {{\n\
         let {binding} = capability.context(missing)?;\n\
         unsafe {{ {body} }}\n\
         }}\n"
    )
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
