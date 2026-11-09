use crate::starlark_document::Document;

#[test]
fn malformed_string_lists_do_not_gain_literal_list_authority() {
    let document = Document::parse("rule(values = [\"entry\")");
    let calls = document.calls("rule");
    assert_eq!(calls.len(), 1);
    assert_eq!(document.keyword_list(&calls[0], "values"), None);
}

#[test]
fn empty_argument_slots_are_not_invented_as_literal_values() {
    let document = Document::parse("rule(,\"first\",,\"second\",)");
    let calls = document.calls("rule");
    let arguments = document.arguments(&calls[0]);
    assert_eq!(arguments.len(), 2);
    assert_eq!(document.string(&arguments[0]), Some("first"));
    assert_eq!(document.string(&arguments[1]), Some("second"));
}

#[test]
fn unterminated_string_tokens_end_at_the_source_boundary() {
    for text in ["\"", "\"value", "'''", "'''value"] {
        let tokens = crate::cargo_starlark_tokens::tokenize(text);
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].string().is_some(), "{text}");
    }
}
