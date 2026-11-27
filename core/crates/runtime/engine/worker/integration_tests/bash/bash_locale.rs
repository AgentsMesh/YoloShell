use super::*;

#[path = "../../test_support/integration/bash/bash_locale.rs"]
mod support;
use self::support::{BashPty, constants::*};

#[tokio::test]
async fn empty_service_environment_preserves_bash_chinese_keyboard_input() {
    let mut shell = BashPty::start(Default::default()).await;
    let diagnostic = shell.query(DIAGNOSTIC_COMMAND, DIAGNOSTIC_END).await;
    assert!(diagnostic.contains(UTF8_CHARMAP), "{diagnostic}");
    assert!(diagnostic.contains(CONVERT_META_OFF), "{diagnostic}");
    assert!(diagnostic.contains(OUTPUT_META_ON), "{diagnostic}");
    shell.send(ASSIGNMENT_PREFIX).await;
    for byte in CHINESE.as_bytes() {
        shell.send(std::slice::from_ref(byte)).await;
    }
    shell.send(ASSIGNMENT_SUFFIX).await;
    assert_exact_value(&mut shell).await;
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn empty_service_environment_preserves_bash_chinese_bracketed_paste() {
    let mut shell = BashPty::start(Default::default()).await;
    let diagnostic = shell.query(DIAGNOSTIC_COMMAND, DIAGNOSTIC_END).await;
    assert!(diagnostic.contains(UTF8_CHARMAP), "{diagnostic}");
    assert!(diagnostic.contains(PASTE_ENABLED), "{diagnostic}");
    shell.send(ASSIGNMENT_PREFIX).await;
    shell.send(PASTE_BEGIN).await;
    shell.send(CHINESE.as_bytes()).await;
    shell.send(PASTE_END).await;
    shell.send(ASSIGNMENT_SUFFIX).await;
    assert_exact_value(&mut shell).await;
}

#[tokio::test]
async fn explicit_c_locale_is_preserved_and_reproduces_readline_numeric_argument() {
    let mut shell = BashPty::start([(LC_ALL_ENV.into(), C_LOCALE.into())].into()).await;
    let diagnostic = shell.query(DIAGNOSTIC_COMMAND, DIAGNOSTIC_END).await;
    assert!(diagnostic.contains(ASCII_CHARMAP), "{diagnostic}");
    assert!(diagnostic.contains(CONVERT_META_ON), "{diagnostic}");
    shell.send(CHINESE.as_bytes()).await;
    shell.screen_contains(ARGUMENT_PROMPT).await;
    // Cancel Readline's numeric argument and clear the pending line; never execute it.
    shell.send(ABORT_LINE).await;
}

async fn assert_exact_value(shell: &mut BashPty) {
    let output = shell.query(VALUE_COMMAND, VALUE_END).await;
    assert!(output.contains(EXPECTED_VALUE), "{output}");
    let hex = output.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(hex.contains(EXPECTED_UTF8_HEX), "{output}");
    assert!(!output.contains(ARGUMENT_PROMPT), "{output}");
}
