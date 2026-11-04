use crate::text::{
    contains_call, contains_identifier, line_count, protocol_shaped, quoted_literals,
};

#[test]
fn counts_physical_source_lines() {
    assert_eq!(line_count(""), 0);
    assert_eq!(line_count("one"), 1);
    assert_eq!(line_count("one\ntwo\n"), 2);
    assert_eq!(line_count("one\n\n"), 2);
}

#[test]
fn extracts_literals_and_ignores_escaped_delimiters() {
    let source = "let first = \"alpha\";\nlet second = \"a\\\"b\";\n";
    let values = quoted_literals(source);
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].value, "alpha");
    assert_eq!(values[1].value, "a\\\"b");
    assert_eq!(values[1].line, 2);
}

#[test]
fn matches_identifiers_calls_and_protocol_shapes() {
    let runtime = ["no", "de"].concat();
    assert!(contains_identifier(&format!("use {runtime};"), &runtime));
    assert!(!contains_identifier(
        &format!("Workspace{runtime}"),
        &runtime
    ));
    assert!(contains_call("window.activate (true)", "activate"));
    let unrelated = ["re", "activate = true"].concat();
    assert!(!contains_call(&unrelated, "activate"));
    assert!(protocol_shaped("writer_changed"));
    assert!(!protocol_shaped("workspace.json"));
}
