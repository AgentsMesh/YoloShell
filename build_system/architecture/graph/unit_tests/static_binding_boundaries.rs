use super::*;
use yolo_architecture_syntax::cargo_starlark_tokens::tokenize;

const ROOT_NAME: &str = "ROOT";

#[test]
fn unfinished_or_multiline_unbounded_assignments_never_resolve_a_crate_root() {
    for text in ["ROOT =", "ROOT = \"src/main.rs\"\n    + \"suffix\""] {
        let bindings = Bindings::parse(&tokenize(text), text);
        assert_eq!(bindings.string(&tokenize(ROOT_NAME)), None, "{text}");
    }
}

#[test]
fn attribute_name_without_assignment_does_not_define_a_root() {
    let tokens = tokenize("rule(crate_root + VALUE)");
    let open = tokens.iter().position(|token| token.symbol('(')).unwrap();
    let close = tokens.len() - NEXT_TOKEN_OFFSET;
    assert!(
        attributes(
            &tokens,
            open,
            close,
            super::super::vocabulary::CRATE_ROOT_ATTRIBUTE
        )
        .is_empty()
    );
}

#[test]
fn scalar_and_list_bindings_remain_distinct() {
    let text = "ROOT = \"src/main.rs\"\nSOURCES = [\"src/main.rs\"]";
    let bindings = Bindings::parse(&tokenize(text), text);
    assert_eq!(bindings.string(&tokenize("SOURCES")), None);
    assert_eq!(bindings.strings(&tokenize(ROOT_NAME)), None);
    assert_eq!(
        bindings.string(&tokenize(ROOT_NAME)).as_deref(),
        Some("src/main.rs")
    );
    assert_eq!(
        bindings.strings(&tokenize("SOURCES")),
        Some(vec!["src/main.rs".into()])
    );
}
