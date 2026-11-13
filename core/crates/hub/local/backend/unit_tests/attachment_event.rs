use super::fixtures::{self, constants::*};
use crate::attachment_event::{connection_loss, convert_event};
use yolo_client::ClientError;
use yolo_hub_terminal_protocol::{PublicError, TerminalEvent};
use yolo_runtime::{
    model::OutputGapPayload,
    protocol::{ProtocolError, event},
};

#[test]
fn actual_exit_and_transport_loss_have_distinct_terminal_events() {
    let exited = convert_event(
        fixtures::target(),
        fixtures::event(event::SESSION_EXITED, &fixtures::exit()),
    )
    .unwrap()
    .unwrap();
    assert!(matches!(
        exited,
        TerminalEvent::Exited { code: Some(0), .. }
    ));
    for error in [
        ClientError::Deadline,
        ClientError::Protocol(ProtocolError::EndOfStream),
        ClientError::Protocol(ProtocolError::Io(
            std::io::ErrorKind::ConnectionReset.into(),
        )),
        ClientError::RuntimeUnavailable {
            terminal_id: fixtures::target().address.terminal_id.as_uuid(),
        },
    ] {
        assert_eq!(
            connection_loss(fixtures::target(), error),
            Ok(TerminalEvent::Unreachable {
                target: fixtures::target()
            })
        );
    }
    assert_eq!(
        connection_loss(
            fixtures::target(),
            ClientError::Protocol(ProtocolError::EventMismatch)
        ),
        Err(PublicError::OutcomeUnknown)
    );
}

#[test]
fn verified_output_and_retention_gap_are_translated() {
    let output = convert_event(
        fixtures::target(),
        fixtures::event(event::TERMINAL_OUTPUT, &fixtures::output()),
    )
    .unwrap()
    .unwrap();
    assert!(matches!(output, TerminalEvent::Output { chunk } if chunk.fields().data == OUTPUT));
    let gap = OutputGapPayload {
        skipped: STEP,
        resync_required: true,
    };
    assert_eq!(
        convert_event(fixtures::target(), fixtures::event(event::OUTPUT_GAP, &gap)),
        Ok(Some(TerminalEvent::ResyncRequired {
            target: fixtures::target()
        }))
    );
    for gap in [
        OutputGapPayload {
            skipped: 0,
            resync_required: true,
        },
        OutputGapPayload {
            skipped: STEP,
            resync_required: false,
        },
    ] {
        assert_eq!(
            convert_event(fixtures::target(), fixtures::event(event::OUTPUT_GAP, &gap)),
            Err(PublicError::InvalidPayload)
        );
    }
    assert_eq!(
        convert_event(fixtures::target(), fixtures::event(event::OUTPUT_GAP, &())),
        Err(PublicError::InvalidPayload)
    );
}

#[test]
fn corrupt_or_wrong_runtime_events_cannot_be_presented_as_current() {
    let mut wrong = fixtures::output();
    wrong.runtime_epoch = uuid::Uuid::new_v4();
    assert_eq!(
        convert_event(
            fixtures::target(),
            fixtures::event(event::TERMINAL_OUTPUT, &wrong)
        ),
        Err(PublicError::TargetMismatch)
    );
    let mut exit = fixtures::exit();
    exit.runtime_epoch = uuid::Uuid::new_v4();
    assert_eq!(
        convert_event(
            fixtures::target(),
            fixtures::event(event::SESSION_EXITED, &exit)
        ),
        Err(PublicError::TargetMismatch)
    );
    let mut output = fixtures::output();
    output.data.clear();
    assert_eq!(
        convert_event(
            fixtures::target(),
            fixtures::event(event::TERMINAL_OUTPUT, &output)
        ),
        Err(PublicError::InvalidPayload)
    );
    let mut envelope = fixtures::event(event::TERMINAL_OUTPUT, &fixtures::output());
    envelope.terminal_id = None;
    assert_eq!(
        convert_event(fixtures::target(), envelope),
        Err(PublicError::TargetMismatch)
    );
}

#[test]
fn non_terminal_side_events_do_not_fabricate_output() {
    assert_eq!(
        convert_event(
            fixtures::target(),
            fixtures::event(event::WRITER_LEASE_ACQUIRED, &())
        ),
        Ok(None)
    );
    let mut chunk = fixtures::output();
    chunk.runtime_epoch = uuid::Uuid::new_v4();
    assert_eq!(
        crate::attachment::chunk_from(fixtures::target(), &chunk),
        Err(PublicError::TargetMismatch)
    );
}
