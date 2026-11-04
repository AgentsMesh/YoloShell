use super::*;

const UNSUPPORTED_SCHEMA_VERSION: u16 = TERMINATION_TOMBSTONE_SCHEMA_VERSION + 1;

#[test]
fn user_request_is_bound_to_schema_terminal_runtime_and_reason() {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let mut tombstone = TerminationTombstone::user_request(terminal_id, runtime_epoch, true);
    assert_eq!(tombstone.validate_for(terminal_id, runtime_epoch), Ok(()));
    tombstone.schema_version = UNSUPPORTED_SCHEMA_VERSION;
    assert_eq!(
        tombstone.validate_for(terminal_id, runtime_epoch),
        Err(TerminationTombstoneError::Schema)
    );
    tombstone.schema_version = TERMINATION_TOMBSTONE_SCHEMA_VERSION;
    assert_eq!(
        tombstone.validate_for(Uuid::new_v4(), runtime_epoch),
        Err(TerminationTombstoneError::Terminal)
    );
    assert_eq!(
        tombstone.validate_for(terminal_id, Uuid::new_v4()),
        Err(TerminationTombstoneError::Runtime)
    );
    tombstone.reason = ExitReason::Natural;
    assert_eq!(
        tombstone.validate_for(terminal_id, runtime_epoch),
        Err(TerminationTombstoneError::Reason)
    );
}
