use super::constants::{MAX_INPUT_BYTES, errors};
use super::wire::CodexHookInput;
use anyhow::{Context, Result, ensure};
use std::io::Read;

pub fn decode(bytes: &[u8]) -> Result<CodexHookInput> {
    ensure!(bytes.len() <= MAX_INPUT_BYTES, errors::INPUT_TOO_LARGE);
    let input: CodexHookInput = serde_json::from_slice(bytes).context(errors::INPUT_INVALID)?;
    input.validate()?;
    Ok(input)
}

pub fn read_stdin() -> Result<CodexHookInput> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take((MAX_INPUT_BYTES.saturating_add(1)) as u64)
        .read_to_end(&mut bytes)
        .context(errors::INPUT_INVALID)?;
    decode(&bytes)
}
