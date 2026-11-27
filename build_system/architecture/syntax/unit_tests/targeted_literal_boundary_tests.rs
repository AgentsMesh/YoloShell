use super::*;

const START: usize = 0;
const FIRST_LINE: usize = 1;
const RAW_IDENTIFIER_END: usize = 6;
const TWO_HASHES: usize = 2;

#[test]
fn raw_identifier_and_lifetime_prefixes_require_complete_identifiers() {
    for source in ["", "x#name", "r", "r#", "r#0x"] {
        assert_eq!(raw_identifier(source, START), None, "{source}");
    }
    assert_eq!(
        raw_identifier("r#type!", START),
        Some(("type", RAW_IDENTIFIER_END))
    );
    for (source, expected) in [
        ("", false),
        ("name", false),
        ("'", false),
        ("'0", false),
        ("'a'", false),
        ("'a", true),
        ("'alpha;", true),
        ("'_", true),
    ] {
        assert_eq!(is_lifetime(source.as_bytes(), START), expected, "{source}");
    }
}

#[test]
fn raw_string_delimiters_do_not_accept_partial_or_mismatched_hashes() {
    for source in ["", "bq", "cq", "r#", "r#name"] {
        assert_eq!(raw_string_prefix(source.as_bytes(), START), None);
    }
    for source in ["r##\"one\"##", "br##\"one\"##", "cr##\"one\"##"] {
        let mut tokens = Vec::new();
        let mut line = FIRST_LINE;
        assert_eq!(
            push_raw_string(source, START, FIRST_LINE, &mut line, &mut tokens),
            Some(source.len())
        );
        assert_eq!(
            tokens,
            vec![Token {
                kind: Kind::String("one".into()),
                line: FIRST_LINE
            }]
        );
    }
    assert!(!has_hashes(b"#", START, TWO_HASHES));
    assert!(!has_hashes(b"#x", START, TWO_HASHES));
    assert!(has_hashes(b"##", START, TWO_HASHES));
}

#[test]
fn unterminated_literals_preserve_the_remaining_source() {
    for (source, expected) in [("r##\"one\"#", "one\"#"), ("r\"one", "one")] {
        let mut tokens = Vec::new();
        let mut line = FIRST_LINE;
        assert_eq!(
            push_raw_string(source, START, FIRST_LINE, &mut line, &mut tokens),
            Some(source.len())
        );
        assert_eq!(
            tokens,
            vec![Token {
                kind: Kind::String(expected.into()),
                line: FIRST_LINE
            }]
        );
    }
    for (source, expected) in [
        ("\"", ""),
        ("\"one", "one"),
        ("\"one\"", "one"),
        ("\"one\\\"two\"", "one\"two"),
    ] {
        let mut tokens = Vec::new();
        let mut line = FIRST_LINE;
        assert_eq!(
            push_string(source, START, FIRST_LINE, &mut line, &mut tokens),
            source.len()
        );
        assert_eq!(
            tokens,
            vec![Token {
                kind: Kind::String(expected.into()),
                line: FIRST_LINE
            }]
        );
    }
}

#[test]
fn nested_and_unterminated_comments_keep_exact_line_accounting() {
    for source in [
        "/* unclosed",
        "/* outer /* nested */",
        "/* outer /* nested */ */",
    ] {
        let mut line = FIRST_LINE;
        assert_eq!(
            skip_block_comment(source.as_bytes(), START, &mut line),
            source.len()
        );
        assert_eq!(line, FIRST_LINE);
    }
    let source = "/* outer\n /* inner */\n */";
    let mut line = FIRST_LINE;
    assert_eq!(
        skip_block_comment(source.as_bytes(), START, &mut line),
        source.len()
    );
    assert_eq!(
        line,
        FIRST_LINE + source.bytes().filter(|byte| *byte == b'\n').count()
    );
}
