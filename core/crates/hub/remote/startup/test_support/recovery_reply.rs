use super::{PORT, State};
use yolo_hub_management_protocol::CONFIGURATION_REVISION_STEP;
use yolo_hub_rpc_protocol::{Command, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

pub(super) fn respond(state: &mut State, request: &Request) -> Option<Reply> {
    state.methods.push(request.fields().command.method());
    match &request.fields().command {
        Command::GetBuild {} => Some(Reply::Build {
            build: state.build.clone(),
        }),
        Command::GetConfiguration {} => {
            if let Some(action) = state.before_snapshot.take() {
                action();
            }
            Some(Reply::Configuration {
                snapshot: state.snapshot.clone(),
            })
        }
        Command::Configure { change } => {
            if change.expected_revision != state.snapshot.state.revision {
                Some(Reply::Error {
                    code: PublicError::Conflict,
                })
            } else {
                state.snapshot.state.configuration = change.configuration.clone();
                state.snapshot.state.last_change = change.request_id;
                state.snapshot.state.revision += CONFIGURATION_REVISION_STEP;
                state.snapshot.listen_address = Some((std::net::Ipv4Addr::LOCALHOST, PORT).into());
                if std::mem::take(&mut state.lose_reply) {
                    None
                } else {
                    Some(Reply::Configuration {
                        snapshot: state.snapshot.clone(),
                    })
                }
            }
        }
        _ => panic!(),
    }
}
