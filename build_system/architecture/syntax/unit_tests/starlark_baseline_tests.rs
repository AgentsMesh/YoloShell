use crate::starlark_baseline::tuple_entries;

#[test]
fn reads_string_tuples_across_lines_and_comments() {
    let source = "\"\"\"Docstring with (\"quoted\", \"words\").\"\"\"\n\
                  BASELINE = [\n\
                  \x20   # (\"commented.rs\", \"out.rs\"),\n\
                  \x20   (\"core/src/lib.rs\", \"part.rs\"),\n\
                  \x20   (\n\
                  \x20       \"core/src/other.rs\",  # trailing comment\n\
                  \x20       \"other.rs\",\n\
                  \x20   ),\n\
                  ]\n";

    let entries = tuple_entries(source, "BASELINE", 2).unwrap();
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert_eq!(entries[0].values, ["core/src/lib.rs", "part.rs"]);
    assert_eq!(entries[1].values, ["core/src/other.rs", "other.rs"]);
}

#[test]
fn rejects_wrong_arity_with_the_entry_line() {
    let source = "BASELINE = [\n    (\"core/src/lib.rs\"),\n]\n";

    let error = tuple_entries(source, "BASELINE", 2).unwrap_err();
    assert_eq!(error.line, 2);
}

#[test]
fn rejects_a_missing_assignment() {
    let source = "OTHER = [\n    (\"core/src/lib.rs\", \"part.rs\"),\n]\n";

    assert!(tuple_entries(source, "BASELINE", 2).is_err());
}

#[test]
fn rejects_a_non_list_assignment() {
    let source = "BASELINE = tuple()\n";

    assert!(tuple_entries(source, "BASELINE", 2).is_err());
}
