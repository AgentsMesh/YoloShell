use super::super::constants::*;
use super::{ControlCapability, frame};
use anyhow::{Context, Result, ensure};
use std::io::Write;

const COMPLETION_FIELD_SEPARATOR_COUNT: usize = 2;
const HEX_DIGITS_PER_BYTE: usize = 2;
const BITS_PER_HEX_DIGIT: u32 = 4;
const LOW_HEX_DIGIT_MASK: u8 = 0x0f;
const CHARACTER_BOUNDARY_STEP: usize = 1;
const HIGH_HEX_DIGIT_INDEX: usize = 0;
const LOW_HEX_DIGIT_INDEX: usize = 1;
const HEX_DIGITS: &[u8] = b"0123456789abcdef";
const COMPLETION_FRAME_FIXED_BYTES: usize = FRAME_COMPLETE.len()
    + CAPABILITY_HEX_LENGTH
    + COMPLETION_FIELD_SEPARATOR_COUNT * FRAME_SEPARATOR.len_utf8()
    + FRAME_TERMINATOR.len_utf8();
const MAXIMUM_COMPLETION_DETAIL_BYTES: usize =
    (MAXIMUM_CONTROL_FRAME_BYTES as usize - COMPLETION_FRAME_FIXED_BYTES) / HEX_DIGITS_PER_BYTE;

#[derive(Debug, Eq, PartialEq)]
pub(in crate::socket_root_guard) struct CompletionFrame {
    pub(in crate::socket_root_guard) completed: bool,
    pub(in crate::socket_root_guard) detail: Option<String>,
}

pub(in crate::socket_root_guard) fn write_completion(
    writer: &mut impl Write,
    capability: &ControlCapability,
    failure: Option<&str>,
) -> Result<()> {
    let (kind, detail) = match failure {
        Some(detail) => {
            ensure!(!detail.is_empty(), WATCHDOG_PROTOCOL_DETAIL);
            (FRAME_FAILED, encode(&bounded(detail)))
        }
        None => (FRAME_COMPLETE, String::new()),
    };
    frame::write_fields(writer, &[kind, capability.value(), &detail])
}

pub(in crate::socket_root_guard) fn parse_completion(
    value: &str,
    capability: &ControlCapability,
) -> Result<CompletionFrame> {
    let fields = frame::parse(value)?;
    let [kind, actual_capability, detail] = fields.as_slice() else {
        anyhow::bail!(WATCHDOG_PROTOCOL_DETAIL);
    };
    ensure!(
        capability.matches(actual_capability),
        WATCHDOG_PROTOCOL_DETAIL
    );
    match *kind {
        FRAME_COMPLETE => {
            ensure!(detail.is_empty(), WATCHDOG_PROTOCOL_DETAIL);
            Ok(CompletionFrame {
                completed: true,
                detail: None,
            })
        }
        FRAME_FAILED => {
            let detail = decode(detail)?;
            ensure!(!detail.is_empty(), WATCHDOG_PROTOCOL_DETAIL);
            Ok(CompletionFrame {
                completed: false,
                detail: Some(detail),
            })
        }
        _ => anyhow::bail!(WATCHDOG_PROTOCOL_DETAIL),
    }
}

fn bounded(detail: &str) -> String {
    if detail.len() <= MAXIMUM_COMPLETION_DETAIL_BYTES {
        return detail.to_owned();
    }
    let maximum_prefix =
        MAXIMUM_COMPLETION_DETAIL_BYTES.saturating_sub(WATCHDOG_OUTPUT_TRUNCATED_SUFFIX.len());
    let mut end = maximum_prefix.min(detail.len());
    while !detail.is_char_boundary(end) {
        end = end.saturating_sub(CHARACTER_BOUNDARY_STEP);
    }
    format!("{}{WATCHDOG_OUTPUT_TRUNCATED_SUFFIX}", &detail[..end])
}

fn encode(detail: &str) -> String {
    let mut encoded = String::with_capacity(detail.len() * HEX_DIGITS_PER_BYTE);
    for byte in detail.bytes() {
        encoded.push(HEX_DIGITS[usize::from(byte >> BITS_PER_HEX_DIGIT)] as char);
        encoded.push(HEX_DIGITS[usize::from(byte & LOW_HEX_DIGIT_MASK)] as char);
    }
    encoded
}

fn decode(encoded: &str) -> Result<String> {
    ensure!(
        encoded.len() % HEX_DIGITS_PER_BYTE == usize::default(),
        WATCHDOG_PROTOCOL_DETAIL
    );
    let bytes = encoded
        .as_bytes()
        .chunks_exact(HEX_DIGITS_PER_BYTE)
        .map(|digits| {
            let high = decode_digit(digits[HIGH_HEX_DIGIT_INDEX])?;
            let low = decode_digit(digits[LOW_HEX_DIGIT_INDEX])?;
            Some((high << BITS_PER_HEX_DIGIT) | low)
        })
        .collect::<Option<Vec<_>>>()
        .context(WATCHDOG_PROTOCOL_DETAIL)?;
    String::from_utf8(bytes).context(WATCHDOG_PROTOCOL_DETAIL)
}

fn decode_digit(value: u8) -> Option<u8> {
    HEX_DIGITS
        .iter()
        .position(|digit| *digit == value)
        .and_then(|position| u8::try_from(position).ok())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/protocol_completion_tests.rs"]
mod tests;
