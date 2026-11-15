use yolo_agent_input::{MAX_PROMPT_BYTES, Prompt};

#[test]
fn preserves_unicode_and_multiline_input_without_interpreting_commands() {
    let text = "你好\n\t$(a command) `literal`";
    let prompt = Prompt::new(text.into()).unwrap();
    assert_eq!(prompt.text(), text);
    let encoded = serde_json::to_vec(&prompt).unwrap();
    assert_eq!(serde_json::from_slice::<Prompt>(&encoded).unwrap(), prompt);
    assert_eq!(String::from(prompt), text);
}

#[test]
fn rejects_terminal_controls_on_both_constructor_and_wire_paths() {
    for text in [
        "",
        " \n\t",
        "a\r",
        "\x1b[201~evil",
        "a\0b",
        "a\x7f",
        "a\u{85}",
    ] {
        let error = Prompt::new(text.into()).unwrap_err();
        assert!(!error.to_string().contains("evil"));
        assert!(serde_json::from_str::<Prompt>(&serde_json::to_string(text).unwrap()).is_err());
    }
}

#[test]
fn enforces_byte_budget_including_multibyte_text() {
    assert!(Prompt::new("a".repeat(MAX_PROMPT_BYTES)).is_ok());
    assert!(Prompt::new("a".repeat(MAX_PROMPT_BYTES + 1)).is_err());
    assert!(Prompt::new("界".repeat(MAX_PROMPT_BYTES / 3 + 1)).is_err());
}
