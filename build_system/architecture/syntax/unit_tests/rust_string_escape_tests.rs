use crate::rust_string_escape::decode;

#[test]
fn decodes_standard_escapes_without_changing_plain_text() {
    for (encoded, decoded) in [
        ("", ""),
        ("plain", "plain"),
        (r"\0\n\r\t", "\0\n\r\t"),
        (r"\\\'", "\\'"),
        (r#"\""#, "\""),
        ("trailing\\", "trailing\\"),
        (r"\q", r"\q"),
        (r"\x41\x00", "A\0"),
        (r"\xGG", r"\xGG"),
        (r"\x", r"\x"),
        (r"\u{41}", "A"),
        (r"\u{0_041}", "A"),
        (r"\u{d800}", r"\u{d800}"),
        (r"\u{110000}", r"\u{110000}"),
        (r"\u{invalid}", r"\u{invalid}"),
        (r"\u{}", r"\u{}"),
        (r"\u", r"\u"),
    ] {
        assert_eq!(decode(encoded), decoded, "{encoded}");
    }
}

#[test]
fn line_continuations_consume_only_following_whitespace() {
    for (encoded, decoded) in [
        ("before\\\n  \t\rafter", "beforeafter"),
        ("\\\n", ""),
        ("\\\n \t", ""),
        ("\\\nafter", "after"),
        ("before\n  after", "before\n  after"),
    ] {
        assert_eq!(decode(encoded), decoded);
    }
}
