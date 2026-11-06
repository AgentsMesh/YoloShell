use crate::command::Command;
use yolo_command_cli::ConnectionOptions;

/// Commands implemented by the auxiliary application crates. They parse here
/// with one root tree and dispatch before any terminal client is started.
pub(crate) enum RootCommand {
    Tui(Command),
    Aux(yolo_command_cli::Command),
}

pub(crate) fn split(command: Command) -> RootCommand {
    use yolo_command_cli::Command as Aux;
    match command {
        Command::Agent { command } => RootCommand::Aux(Aux::Agent { command }),
        Command::Conversation { action } => RootCommand::Aux(Aux::Conversation { action }),
        Command::Workspace { command } => RootCommand::Aux(Aux::Workspace { command }),
        Command::Worktree { command } => RootCommand::Aux(Aux::Worktree { command }),
        Command::Protocol { command } => RootCommand::Aux(Aux::Protocol { command }),
        Command::Plugin { command } => RootCommand::Aux(Aux::Plugin { command }),
        Command::Update(args) => RootCommand::Aux(Aux::Update(args)),
        command => RootCommand::Tui(command),
    }
}

pub(crate) fn run(connection: ConnectionOptions, command: yolo_command_cli::Command) -> anyhow::Result<()> {
    yolo_command_cli::run(connection, command)
}
