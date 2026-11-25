use super::*;

const INITIAL_DEPTH: usize = 0;

#[test]
fn exhausted_shell_recursion_rejects_even_an_otherwise_empty_nested_script() {
    assert!(contains_signal_command("", MAX_NESTED_SHELLS));
    assert!(!contains_signal_command("", INITIAL_DEPTH));
    assert!(!segment_is_signal(&[], INITIAL_DEPTH));
    for (text, expected) in [
        ("alias callback=/bin/kill", true),
        ("alias clean=echo", false),
        ("NAME=value", false),
        ("env NAME=value /bin/kill 1", true),
        ("sh script.sh", false),
        ("sh -x script.sh", false),
        ("sh -c 'echo safe'", false),
        ("sh -c 'kill 1'", true),
        ("echo $(echo $(kill 1))", true),
    ] {
        assert_eq!(
            contains_signal_command(text, INITIAL_DEPTH),
            expected,
            "{text}"
        );
    }
}

#[test]
fn shell_assignment_detection_requires_both_name_and_value() {
    for (word, expected) in [
        ("NAME=value", Some("value")),
        ("NAME=a=b", Some("a=b")),
        ("NAME=", None),
        ("=value", None),
        ("NAME", None),
    ] {
        assert_eq!(assignment(word), expected, "{word}");
    }
    let words = ["FIRST=value", "SECOND=value", "echo"].map(str::to_owned);
    assert_eq!(skip_assignments(&words, INITIAL_DEPTH), words.len() - 1);
    assert_eq!(skip_assignments(&words, words.len()), words.len());
    assert_eq!(basename("/"), "/");
}
