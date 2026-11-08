use crate::constants::*;
use crate::record::{AcceptanceCheckResult, AcceptancePlatform, AcceptanceProviderVersion};
use anyhow::{Result, ensure};

pub(crate) fn validate_platform(value: &AcceptancePlatform) -> Result<()> {
    ensure!(
        value.name.len() <= ACCEPTANCE_PLATFORM_NAME_MAX_BYTES,
        RECEIPT_FIELD_ERROR
    );
    validate_text(&value.name)?;
    validate_text(&value.version)?;
    validate_text(&value.architecture)?;
    ensure!(
        value.name == ACCEPTANCE_PLATFORM_NAME
            && value.architecture == ACCEPTANCE_PLATFORM_ARCHITECTURE,
        RECEIPT_TARGET_ERROR
    );
    ensure!(
        is_supported_macos_version(&value.version),
        RECEIPT_TARGET_ERROR
    );
    Ok(())
}

pub(crate) fn validate_provider(value: &AcceptanceProviderVersion) -> Result<()> {
    validate_text(&value.provider)?;
    validate_text(&value.version)?;
    let expected_output = match value.provider.as_str() {
        CLAUDE_PROVIDER => CLAUDE_ACCEPTANCE_VERSION_OUTPUT,
        CODEX_PROVIDER => CODEX_ACCEPTANCE_VERSION_OUTPUT,
        _ => return Err(anyhow::anyhow!(RECEIPT_TARGET_ERROR)),
    };
    ensure!(value.version == expected_output, RECEIPT_TARGET_ERROR);
    Ok(())
}

pub(crate) fn validate_binding_text(value: &str) -> Result<()> {
    validate_text(value)
}

pub(crate) fn validate_digest(value: &str) -> Result<()> {
    ensure!(value.len() == SHA256_HEX_CHARACTERS, RECEIPT_DIGEST_ERROR);
    ensure!(
        value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        RECEIPT_DIGEST_ERROR
    );
    Ok(())
}

pub(crate) fn validate_providers(values: &[AcceptanceProviderVersion]) -> Result<()> {
    ensure!(
        values.len() <= MAX_ACCEPTANCE_PROVIDER_ENTRIES,
        RECEIPT_PROVIDER_ERROR
    );
    for value in values {
        validate_provider(value)?;
    }
    ensure!(
        values.len() == REQUIRED_ACCEPTANCE_PROVIDERS.len(),
        RECEIPT_PROVIDER_ERROR
    );
    ensure!(
        values
            .iter()
            .zip(REQUIRED_ACCEPTANCE_PROVIDERS)
            .all(|(value, expected)| value.provider == *expected),
        RECEIPT_PROVIDER_ERROR
    );
    Ok(())
}

pub(crate) fn validate_results(values: &[AcceptanceCheckResult]) -> Result<()> {
    ensure!(
        values.len() == REQUIRED_ACCEPTANCE_CHECKS.len(),
        RECEIPT_CHECKLIST_ERROR
    );
    for (result, expected) in values.iter().zip(REQUIRED_ACCEPTANCE_CHECKS) {
        validate_check_result(result)?;
        ensure!(result.identifier == *expected, RECEIPT_CHECKLIST_ERROR);
    }
    Ok(())
}

pub(crate) fn validate_check_result(value: &AcceptanceCheckResult) -> Result<()> {
    validate_text(&value.identifier)?;
    ensure!(
        value.status == ACCEPTANCE_CHECK_STATUS_PASSED,
        RECEIPT_CHECKLIST_ERROR
    );
    Ok(())
}

fn validate_text(value: &str) -> Result<()> {
    ensure!(!value.is_empty(), RECEIPT_FIELD_ERROR);
    ensure!(
        value.len() <= MAX_ACCEPTANCE_FIELD_BYTES,
        RECEIPT_FIELD_ERROR
    );
    ensure!(
        value.chars().all(|character| !character.is_control()),
        RECEIPT_FIELD_ERROR
    );
    Ok(())
}

fn is_supported_macos_version(value: &str) -> bool {
    let components: Vec<_> = value.split('.').collect();
    if !(MIN_MACOS_VERSION_COMPONENTS..=MAX_MACOS_VERSION_COMPONENTS).contains(&components.len()) {
        return false;
    }
    if components.iter().any(|component| {
        component.is_empty()
            || (component.len() > 1 && component.starts_with('0'))
            || component.parse::<u16>().is_err()
    }) {
        return false;
    }
    let major = components[0].parse::<u16>().expect("validated macOS major");
    let minor = components[1].parse::<u16>().expect("validated macOS minor");
    major > MIN_MACOS_MAJOR_VERSION
        || (major == MIN_MACOS_MAJOR_VERSION && minor >= MIN_MACOS_MINOR_VERSION)
}
