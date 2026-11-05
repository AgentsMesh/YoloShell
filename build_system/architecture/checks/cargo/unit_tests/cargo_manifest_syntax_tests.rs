use crate::cargo_manifest_syntax::{Value, parse};

#[test]
fn declarations_keep_source_lines_table_ownership_and_array_kind() {
    let parsed =
        parse("# ignored\n[ Alpha . Beta ]\nfield = false # comment\n[[items]]\nfield = 'text'\n");
    assert!(parsed.unsupported_lines.is_empty());
    assert_eq!(parsed.tables.len(), 2);
    assert_eq!(
        (
            &*parsed.tables[0].name,
            parsed.tables[0].line,
            parsed.tables[0].array
        ),
        ("alpha.beta", 2, false)
    );
    assert_eq!(
        (
            &*parsed.tables[1].name,
            parsed.tables[1].line,
            parsed.tables[1].array
        ),
        ("items", 4, true)
    );
    assert_eq!(parsed.entries[0].table, "alpha.beta");
    assert_eq!(parsed.entries[0].key, "field");
    assert_eq!(parsed.entries[0].line, 3);
    assert_eq!(parsed.entries[0].value, Value::False);
    assert_eq!(parsed.entries[1].value, Value::Text("text".to_owned()));
}

#[test]
fn unsupported_headers_keys_and_multiline_quoting_are_reported_at_the_source() {
    for source in [
        "[unfinished",
        "[]",
        "[[unfinished]",
        "[invalid/name]",
        r"[escaped\name]",
        "'' = false",
        "invalid/key = false",
        r"escaped\key = false",
        "field = '''multiline'''",
        "field = \"\"\"multiline\"\"\"",
        "field = 'unfinished",
        "field = \"unfinished",
        "field = \"escaped\\\"",
    ] {
        let parsed = parse(source);
        assert_eq!(parsed.unsupported_lines, [1], "{source}");
        assert!(parsed.entries.is_empty(), "{source}");
    }
}

#[test]
fn assignment_and_comment_delimiters_inside_quotes_are_not_syntax() {
    for (source, expected) in [
        (
            "field = 'left#right' # outside",
            Value::Text("left#right".to_owned()),
        ),
        (
            "field = \"left=right\"",
            Value::Text("left=right".to_owned()),
        ),
        (
            r#"field = 'literal\path'"#,
            Value::Text(r"literal\path".to_owned()),
        ),
        (r#"field = "escaped\path""#, Value::Other),
        ("field = 123", Value::Other),
        ("field = true", Value::Other),
    ] {
        let parsed = parse(source);
        assert!(parsed.unsupported_lines.is_empty(), "{source}");
        assert_eq!(parsed.entries[0].value, expected, "{source}");
    }
    for source in [
        "'not=assignment'",
        "\"not=assignment\"",
        r#""not\"=assignment""#,
        "not_assignment",
    ] {
        assert!(parse(source).entries.is_empty(), "{source}");
    }
}
