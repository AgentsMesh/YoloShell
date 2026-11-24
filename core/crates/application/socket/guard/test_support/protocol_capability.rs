use super::super::constants::*;
use anyhow::{Result, ensure};

#[derive(Clone)]
pub(crate) struct ControlCapability(String);

#[derive(Clone)]
pub(crate) struct DiscoveryCapability(String);

impl ControlCapability {
    pub(crate) fn generate() -> Self {
        Self(generate_secret())
    }

    pub(crate) fn parse(value: &str) -> Result<Self> {
        validate_secret(value)?;
        Ok(Self(value.to_owned()))
    }

    pub(crate) fn value(&self) -> &str {
        &self.0
    }

    pub(crate) fn matches(&self, candidate: &str) -> bool {
        let left = self.0.as_bytes();
        let right = candidate.as_bytes();
        let mut difference = left.len() ^ right.len();
        for index in 0..left.len() {
            difference |= usize::from(left[index] ^ right.get(index).copied().unwrap_or_default());
        }
        difference == usize::default()
    }
}

impl DiscoveryCapability {
    #[cfg(test)]
    pub(crate) fn generate() -> Self {
        Self(generate_secret())
    }

    pub(crate) fn parse(value: &str) -> Result<Self> {
        validate_secret(value)?;
        Ok(Self(value.to_owned()))
    }

    pub(crate) fn value(&self) -> &str {
        &self.0
    }
}

fn generate_secret() -> String {
    std::iter::repeat_with(uuid::Uuid::new_v4)
        .take(CAPABILITY_UUID_PARTS)
        .map(|part| part.simple().to_string())
        .collect()
}

fn validate_secret(value: &str) -> Result<()> {
    ensure!(
        value.len() == CAPABILITY_HEX_LENGTH
            && value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        WATCHDOG_PROTOCOL_DETAIL
    );
    Ok(())
}
