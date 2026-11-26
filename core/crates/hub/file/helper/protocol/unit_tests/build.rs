use crate::fixtures::*;
use yolo_hub_file_helper_protocol::{Command, Request};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn build_requires_no_file_authority_and_cannot_launder_it_into_a_read() {
    let mut fields = build();
    fields.protected.push(PROTECTED_PATH.into());
    assert_eq!(Request::new(fields), Err(PublicError::InvalidRequest));
    let mut fields = build();
    fields.command = Command::Inspect { roots: vec![] };
    assert_eq!(Request::new(fields), Err(PublicError::InvalidRequest));
    let mut fields = build();
    fields.command = read().command;
    assert_eq!(Request::new(fields), Err(PublicError::InvalidRequest));
}
