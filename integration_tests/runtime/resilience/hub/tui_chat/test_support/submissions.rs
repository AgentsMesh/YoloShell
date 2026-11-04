const RECORD_END: u8 = b'\n';

/// The fake provider commits each appended JSON string with a newline. Its
/// unfinished tail can stop within a UTF-8 character or JSON escape sequence.
/// Complete malformed records remain errors, never silently skipped evidence.
pub fn committed(bytes: &[u8]) -> Result<Vec<String>, serde_json::Error> {
    bytes
        .split_inclusive(|byte| *byte == RECORD_END)
        .filter(|record| record.last() == Some(&RECORD_END))
        .map(serde_json::from_slice)
        .collect()
}
