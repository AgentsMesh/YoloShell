use super::acceptance_receipt_support::{
    CLAUDE_PROVIDER_INDEX, CODEX_PROVIDER_INDEX, LEGACY_CLAUDE_OUTPUT_PREFIX,
    LEGACY_CODEX_OUTPUT_PREFIX, PROVIDER_CRLF, PROVIDER_LEADING_SPACE, PROVIDER_LINE_FEED,
    PROVIDER_VERSION_SUFFIX, record, try_signed,
};
use crate::constants::{
    CLAUDE_ACCEPTANCE_VERSION, CLAUDE_ACCEPTANCE_VERSION_OUTPUT, CODEX_ACCEPTANCE_VERSION,
    CODEX_ACCEPTANCE_VERSION_OUTPUT,
};

const PROVIDER_TAB: &str = "\t";
const PROVIDER_EXTRA_PREFIX: &str = "version: ";
const PROVIDER_EXTRA_SUFFIX: &str = " extra-output";
const PROVIDER_DOUBLE_SPACE: &str = "  ";
const NEIGHBORING_CLAUDE_OUTPUT: &str = "2.1.242 (Claude Code)";
const LOWER_NEIGHBORING_CODEX_OUTPUT: &str = "codex-cli 0.150.0";
const UPPER_NEIGHBORING_CODEX_OUTPUT: &str = "codex-cli 0.150.2";

#[test]
fn exact_provider_outputs_are_accepted() {
    assert!(try_signed(record()).is_ok());
}

#[test]
fn bare_provider_versions_are_rejected() {
    for (index, version) in [
        (CLAUDE_PROVIDER_INDEX, CLAUDE_ACCEPTANCE_VERSION),
        (CODEX_PROVIDER_INDEX, CODEX_ACCEPTANCE_VERSION),
    ] {
        let mut value = record();
        value.providers[index].version = version.to_owned();
        assert!(try_signed(value).is_err());
    }
}

#[test]
fn legacy_provider_outputs_are_rejected() {
    for (index, version) in [
        (
            CLAUDE_PROVIDER_INDEX,
            format!("{LEGACY_CLAUDE_OUTPUT_PREFIX}{CLAUDE_ACCEPTANCE_VERSION}"),
        ),
        (
            CODEX_PROVIDER_INDEX,
            format!("{LEGACY_CODEX_OUTPUT_PREFIX}{CODEX_ACCEPTANCE_VERSION}"),
        ),
    ] {
        let mut value = record();
        value.providers[index].version = version;
        assert!(try_signed(value).is_err());
    }
}

#[test]
fn provider_output_whitespace_and_line_endings_are_rejected() {
    for (index, exact) in provider_outputs() {
        for invalid in [
            format!("{PROVIDER_LEADING_SPACE}{exact}"),
            format!("{exact}{PROVIDER_LEADING_SPACE}"),
            format!("{exact}{PROVIDER_LINE_FEED}"),
            format!("{exact}{PROVIDER_CRLF}"),
        ] {
            let mut value = record();
            value.providers[index].version = invalid;
            assert!(try_signed(value).is_err());
        }
    }
}

#[test]
fn provider_output_suffixes_are_rejected() {
    for (index, exact) in provider_outputs() {
        let mut value = record();
        value.providers[index].version = format!("{exact}{PROVIDER_VERSION_SUFFIX}");
        assert!(try_signed(value).is_err());
    }
}

#[test]
fn provider_output_tabs_and_internal_spacing_are_rejected() {
    for (index, exact) in provider_outputs() {
        for invalid in [
            format!("{PROVIDER_TAB}{exact}"),
            format!("{exact}{PROVIDER_TAB}"),
            exact.replacen(' ', PROVIDER_DOUBLE_SPACE, 1),
            exact.replacen(' ', PROVIDER_TAB, 1),
        ] {
            assert_output_rejected(index, invalid);
        }
    }
}

#[test]
fn provider_output_case_and_extra_text_are_rejected() {
    for (index, exact) in provider_outputs() {
        for invalid in [
            exact.to_ascii_uppercase(),
            format!("{PROVIDER_EXTRA_PREFIX}{exact}"),
            format!("{exact}{PROVIDER_EXTRA_SUFFIX}"),
        ] {
            assert_output_rejected(index, invalid);
        }
    }
}

#[test]
fn neighboring_provider_versions_are_rejected() {
    for (index, invalid) in [
        (CLAUDE_PROVIDER_INDEX, NEIGHBORING_CLAUDE_OUTPUT),
        (CODEX_PROVIDER_INDEX, LOWER_NEIGHBORING_CODEX_OUTPUT),
        (CODEX_PROVIDER_INDEX, UPPER_NEIGHBORING_CODEX_OUTPUT),
    ] {
        assert_output_rejected(index, invalid.to_owned());
    }
}

#[test]
fn cross_provider_output_bindings_are_rejected() {
    let mut claude = record();
    claude.providers[CLAUDE_PROVIDER_INDEX].version = CODEX_ACCEPTANCE_VERSION_OUTPUT.to_owned();
    assert!(try_signed(claude).is_err());

    let mut codex = record();
    codex.providers[CODEX_PROVIDER_INDEX].version = CLAUDE_ACCEPTANCE_VERSION_OUTPUT.to_owned();
    assert!(try_signed(codex).is_err());
}

fn provider_outputs() -> impl Iterator<Item = (usize, &'static str)> {
    [
        (CLAUDE_PROVIDER_INDEX, CLAUDE_ACCEPTANCE_VERSION_OUTPUT),
        (CODEX_PROVIDER_INDEX, CODEX_ACCEPTANCE_VERSION_OUTPUT),
    ]
    .into_iter()
}

fn assert_output_rejected(index: usize, invalid: String) {
    let mut value = record();
    value.providers[index].version = invalid;
    assert!(try_signed(value).is_err());
}
