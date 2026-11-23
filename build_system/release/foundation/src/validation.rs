use anyhow::{Result, ensure};

pub const GIT_SHA1_HEX_LENGTH: usize = 40;
pub const GIT_SHA256_HEX_LENGTH: usize = 64;
pub const SHA256_HEX_LENGTH: usize = GIT_SHA256_HEX_LENGTH;

pub fn validate_revision(value: &str) -> Result<()> {
    ensure!(
        matches!(value.len(), GIT_SHA1_HEX_LENGTH | GIT_SHA256_HEX_LENGTH) && lowercase_hex(value),
        "source revision must be a lowercase Git object ID"
    );
    Ok(())
}

pub fn validate_sha256(value: &str) -> Result<()> {
    ensure!(
        value.len() == SHA256_HEX_LENGTH && lowercase_hex(value),
        "invalid SHA-256 digest"
    );
    Ok(())
}

fn lowercase_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
