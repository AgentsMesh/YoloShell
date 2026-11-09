use crate::cargo_shell_substitutions::substitutions;

#[test]
fn single_quoted_backslashes_do_not_hide_substitutions_or_their_closing_parenthesis() {
    for (source, body) in [
        (r"'literal\' $(actual)", "actual"),
        (r"$(printf '%s' 'literal\')", r"printf '%s' 'literal\'"),
    ] {
        let entries = substitutions(source);
        assert_eq!(entries.len(), 1, "{source}");
        assert_eq!(entries[0].source, body, "{source}");
    }
}

#[test]
fn cargo_execution_is_detected_after_a_literal_backslash_and_inside_double_quotes() {
    let executable = crate::policy_constants::CARGO_EXECUTABLE;
    let source = format!(r#"printf '%s' 'literal\' "$({executable} metadata)""#);
    assert_eq!(crate::cargo_shell::execution_lines(&source), [1]);
}

#[test]
fn nested_and_quoted_substitutions_keep_exact_body_and_start_line() {
    for (source, body) in [
        ("$(outer (inner))", "outer (inner)"),
        ("$(outer ')')", "outer ')'"),
        ("$(outer \"(\")", "outer \"(\""),
        (r"$(outer \))", r"outer \)"),
        (r"`outer \` inner`", r"outer \` inner"),
        ("\"$(outer 'inner')\"", "outer 'inner'"),
        ("$(outer \"'inner'\")", "outer \"'inner'\""),
    ] {
        let entries = substitutions(source);
        assert_eq!(entries.len(), 1, "{source}");
        assert_eq!(entries[0].source, body, "{source}");
        assert_eq!(entries[0].line, 1);
    }
}

#[test]
fn literals_escapes_unclosed_delimiters_and_comments_do_not_create_commands() {
    for source in [
        "'$(literal)'",
        "'`literal`'",
        r"\$(escaped)",
        r"\`escaped\`",
        "`unfinished",
        "$(unfinished",
        "# $(comment)",
        " ;# $(comment)",
        " |# $(comment)",
        " &# $(comment)",
        "'double \" $(literal)'",
        "trailing\\",
    ] {
        assert!(substitutions(source).is_empty(), "{source}");
    }
    let entries = substitutions("prefix#$(actual)");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "actual");
}

#[test]
fn whitespace_comments_do_not_turn_embedded_substitutions_into_executions() {
    let executable = crate::policy_constants::CARGO_EXECUTABLE;
    for whitespace in [" ", "\t"] {
        let source = format!("printf value{whitespace}# $({executable})\n$(actual)");
        let entries = substitutions(&source);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "actual");
        assert_eq!(entries[0].line, 2);
        assert!(crate::cargo_shell::execution_lines(&source).is_empty());
    }
}

#[test]
fn multiline_bodies_comments_and_escaped_newlines_preserve_following_locations() {
    let entries = substitutions("# ignored\n\\\n$(first\nbody)\n`second\nbody`\n$(last)");
    assert_eq!(
        entries.iter().map(|entry| entry.line).collect::<Vec<_>>(),
        [3, 5, 7]
    );
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.source.as_str())
            .collect::<Vec<_>>(),
        ["first\nbody", "second\nbody", "last"]
    );
}
