use super::constants::*;
use crate::arguments::Arguments;
use clap::Parser;

#[test]
fn json_flag_is_accepted_for_protocol_discovery() {
    let plain = vec![PROGRAM, crate::PROTOCOL_COMMAND, DESCRIBE];
    let explicit = vec![PROGRAM, crate::PROTOCOL_COMMAND, DESCRIBE, JSON_ARGUMENT];
    let mut plain_bytes = Vec::new();
    crate::output::write(
        Arguments::try_parse_from(&plain).unwrap().command,
        &mut plain_bytes,
    )
    .unwrap();
    let mut explicit_bytes = Vec::new();
    crate::output::write(
        Arguments::try_parse_from(&explicit).unwrap().command,
        &mut explicit_bytes,
    )
    .unwrap();
    assert_eq!(plain_bytes, explicit_bytes);
}
