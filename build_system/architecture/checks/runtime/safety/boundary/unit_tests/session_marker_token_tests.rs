use super::*;
use yolo_architecture_syntax::targeted_tokens::Kind;

#[test]
fn session_marker_definition_requires_every_scoped_visibility_token() {
    let tokens = tokenize(&format!("pub(super) fn {SESSION_MARKER_CAS_WRITER}"));
    let definition = tokens.len() - 1;
    assert!(is_scoped_definition(&tokens, definition));
    for index in 0..definition {
        for kind in [
            Kind::Identifier("other".into()),
            Kind::Symbol("other".into()),
        ] {
            let mut changed = tokens.clone();
            changed[index].kind = kind;
            assert!(!is_scoped_definition(&changed, definition), "{index}");
        }
    }
    for index in 0..definition {
        assert!(!is_scoped_definition(&tokens, index));
    }
}

#[test]
fn session_marker_owner_does_not_admit_aliases_non_calls_or_neighboring_functions() {
    for (body, expected) in [
        (format!("{SESSION_MARKER_CAS_WRITER}()"), true),
        (SESSION_MARKER_CAS_WRITER.to_owned(), false),
        (format!("use {SESSION_MARKER_CAS_WRITER}()"), false),
        (format!("value as {SESSION_MARKER_CAS_WRITER}()"), false),
    ] {
        let text = format!("fn {SESSION_MARKER_CAS_OWNER_FUNCTION}() {{ {body}; }}");
        let owner_file = file(OWNER_PATH, &text);
        let tokens = tokenize(&text);
        let call = tokens
            .iter()
            .position(|token| token.identifier(SESSION_MARKER_CAS_WRITER))
            .unwrap();
        assert_eq!(
            allowed_owner_call(&owner_file, &tokens, call),
            expected,
            "{body}"
        );
        assert!(!allowed_owner_call(&file("other.rs", &text), &tokens, call));
    }
    for text in [
        format!(
            "fn {SESSION_MARKER_CAS_OWNER_FUNCTION}() {{}} fn other() {{ {SESSION_MARKER_CAS_WRITER}(); }}"
        ),
        format!("fn {SESSION_MARKER_CAS_OWNER_FUNCTION}(); {SESSION_MARKER_CAS_WRITER}();"),
    ] {
        let tokens = tokenize(&text);
        let call = tokens
            .iter()
            .position(|token| token.identifier(SESSION_MARKER_CAS_WRITER))
            .unwrap();
        assert!(!allowed_owner_call(&file(OWNER_PATH, &text), &tokens, call));
    }
}

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: path.into(),
        relative: path.into(),
        text: text.into(),
    }
}
