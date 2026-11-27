use super::fixtures::*;
use crate::{Arguments, constants::*, status::Frame};
use clap::Parser;
use yolo_hub_terminal_relay::{ControlIntent, Status};

pub(super) fn arguments() -> Vec<String> {
    let mut args = vec![PROGRAM.into()];
    for (option, value) in [
        (SOCKET_OPTION, SOCKET),
        (HUB_OPTION, HUB),
        (ORIGIN_OPTION, ORIGIN),
        (TERMINAL_OPTION, SESSION),
        (EPOCH_OPTION, EPOCH),
        (HOLDER_OPTION, HOLDER),
    ] {
        args.extend([format!("{OPTION_PREFIX}{option}"), value.into()]);
    }
    args
}

#[test]
fn relay_arguments_preserve_entry_origin_exact_epoch_and_unique_controller_instance() {
    let args = Arguments::try_parse_from(arguments()).unwrap();
    let first = args.options(None);
    let second = args.options(None);
    assert_eq!(first.entry.to_string(), HUB);
    assert_eq!(first.target.address.hub_id.to_string(), ORIGIN);
    assert_eq!(first.target.address.terminal_id.to_string(), SESSION);
    assert_eq!(first.target.runtime_epoch.to_string(), EPOCH);
    assert_eq!(first.principal.client_id.to_string(), HOLDER);
    assert_ne!(
        first.principal.controller_instance_id,
        second.principal.controller_instance_id
    );
    assert_eq!(first.intent, ControlIntent::Acquire);
    let mut observer_arguments = arguments();
    observer_arguments.push(format!("{OPTION_PREFIX}{OBSERVE_OPTION}"));
    assert_eq!(
        Arguments::try_parse_from(&observer_arguments)
            .unwrap()
            .options(None)
            .intent,
        ControlIntent::Observe
    );
    observer_arguments.push(format!("{OPTION_PREFIX}{TAKEOVER_OPTION}"));
    assert!(Arguments::try_parse_from(observer_arguments).is_err());
}

#[test]
fn status_frames_only_expose_existing_native_fields_without_writer_capabilities() {
    for (status, event, holder) in [
        (Status::ControlAcquired, CONTROL_ACQUIRED, None),
        (
            Status::ReadOnly {
                holder_id: Some(OTHER_HOLDER.into()),
            },
            READ_ONLY,
            Some(OTHER_HOLDER),
        ),
        (Status::ControlLost { holder_id: None }, CONTROL_LOST, None),
        (Status::SessionExited, SESSION_EXITED, None),
    ] {
        let frame = Frame::from(status);
        assert_eq!(frame.event, event);
        assert_eq!(frame.holder_id.as_deref(), holder);
    }
}
