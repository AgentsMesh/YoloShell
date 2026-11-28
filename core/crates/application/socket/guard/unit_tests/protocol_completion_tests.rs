use super::super::write_capability;
use super::*;
use std::io::Write;

const FAILURE_DETAIL: &str = "watchdog failure\twith\nprotocol separators";
const REJECTED_WRITE_DETAIL: &str = "injected completion write failure";
const EMPTY_DETAIL: &str = "";
const ODD_HEX_DETAIL: &str = "0";
const UPPERCASE_HEX_DETAIL: &str = "AA";
const INVALID_UTF8_HEX_DETAIL: &str = "ff";
const NONEMPTY_COMPLETE_HEX_DETAIL: &str = "61";
const OVERSIZED_DETAIL_BYTE: char = 'x';
const OVERSIZED_DETAIL_INCREMENT: usize = 1;

#[test]
fn authenticated_completion_preserves_failure_detail() {
    let capability = ControlCapability::generate();
    let mut completed = Vec::new();
    write_completion(&mut completed, &capability, None).unwrap();
    assert_eq!(
        parse_completion(&String::from_utf8(completed).unwrap(), &capability).unwrap(),
        CompletionFrame {
            completed: true,
            detail: None,
        }
    );

    let mut failed = Vec::new();
    write_completion(&mut failed, &capability, Some(FAILURE_DETAIL)).unwrap();
    assert_eq!(
        parse_completion(&String::from_utf8(failed).unwrap(), &capability).unwrap(),
        CompletionFrame {
            completed: false,
            detail: Some(FAILURE_DETAIL.to_owned()),
        }
    );
}

#[test]
fn completion_rejects_wrong_authority_kind_and_detail_shape() {
    let capability = ControlCapability::generate();
    let other = ControlCapability::generate();
    let mut failed = Vec::new();
    write_completion(&mut failed, &capability, Some(FAILURE_DETAIL)).unwrap();
    let failed = String::from_utf8(failed).unwrap();
    assert!(parse_completion(&failed, &other).is_err());

    let mut finish = Vec::new();
    write_capability(&mut finish, FRAME_FINISH, &capability).unwrap();
    assert!(parse_completion(&String::from_utf8(finish).unwrap(), &capability).is_err());
    assert!(write_completion(&mut Vec::new(), &capability, Some(EMPTY_DETAIL)).is_err());
}

#[test]
fn oversized_completion_detail_is_bounded_and_marks_truncation() {
    let detail = OVERSIZED_DETAIL_BYTE
        .to_string()
        .repeat(MAXIMUM_COMPLETION_DETAIL_BYTES + OVERSIZED_DETAIL_INCREMENT);
    let capability = ControlCapability::generate();
    let mut encoded = Vec::new();
    write_completion(&mut encoded, &capability, Some(&detail)).unwrap();
    assert!(u64::try_from(encoded.len()).unwrap() <= MAXIMUM_CONTROL_FRAME_BYTES);

    let parsed = parse_completion(&String::from_utf8(encoded).unwrap(), &capability).unwrap();
    let parsed = parsed.detail.unwrap();
    assert!(parsed.ends_with(WATCHDOG_OUTPUT_TRUNCATED_SUFFIX));
    assert!(parsed.len() <= MAXIMUM_COMPLETION_DETAIL_BYTES);
}

#[test]
fn completion_rejects_malformed_hex_and_complete_detail() {
    let capability = ControlCapability::generate();
    for (kind, detail) in [
        (FRAME_FAILED, ODD_HEX_DETAIL),
        (FRAME_FAILED, UPPERCASE_HEX_DETAIL),
        (FRAME_FAILED, INVALID_UTF8_HEX_DETAIL),
        (FRAME_COMPLETE, NONEMPTY_COMPLETE_HEX_DETAIL),
    ] {
        let mut encoded = Vec::new();
        frame::write_fields(&mut encoded, &[kind, capability.value(), detail]).unwrap();
        assert!(parse_completion(&String::from_utf8(encoded).unwrap(), &capability).is_err());
    }
}

#[test]
fn completion_write_failure_is_not_discarded() {
    let capability = ControlCapability::generate();
    let error = write_completion(&mut RejectWrites, &capability, Some(FAILURE_DETAIL)).unwrap_err();
    assert!(format!("{error:#}").contains(REJECTED_WRITE_DETAIL));
}

struct RejectWrites;

impl Write for RejectWrites {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other(REJECTED_WRITE_DETAIL))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
