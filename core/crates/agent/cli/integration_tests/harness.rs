use crate::arguments::{AgentCommand, Arguments, Command, Options, OutputArgs};
use crate::outcome::Outcome;
use std::path::PathBuf;
use yolo_hub_identity::HubId;

pub fn arguments(socket: PathBuf, hub: HubId, command: AgentCommand) -> Arguments {
    Arguments {
        command: Command::Agent(Options {
            socket: Some(socket),
            hub: Some(hub),
            local_profile: None,
            command,
        }),
    }
}

pub async fn run(socket: PathBuf, hub: HubId, command: AgentCommand) -> Outcome {
    crate::runner::run(arguments(socket, hub, command))
        .await
        .unwrap()
}

pub fn output(json: bool) -> OutputArgs {
    OutputArgs { json }
}
