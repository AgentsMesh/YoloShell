use crate::terminal::TerminalCommand as CliTerminalCommand;
use crate::constants::*;
use anyhow::{Context, Result, ensure};
use std::io::{self, Write};
use yolo_hub_rpc_protocol::{Command as HubCommand, Reply};
use yolo_hub_terminal_protocol::{
    RuntimeStatus, TerminalAction, TerminalCommand as HubTerminalCommand, TerminalSelection,
    TerminalSummary,
};
use yolo_tui_app::Options;

pub(crate) async fn run(options: &Options, command: CliTerminalCommand) -> Result<()> {
    let (selector, action, json) = match command {
        CliTerminalCommand::Archive { terminal, json } => {
            (terminal, TerminalAction::Archive {}, json)
        }
        CliTerminalCommand::Terminate {
            terminal,
            force,
            json,
        } => (terminal, TerminalAction::Terminate { force }, json),
        _ => unreachable!("only lifecycle commands use the lifecycle path"),
    };
    let read_deadline = tokio::time::Instant::now() + CATALOG_TIMEOUT;
    let mut client = options.endpoint.connect(options.hub, read_deadline).await?;
    let catalog =
        yolo_hub_catalog_client::read_with_archived(&mut client, read_deadline, false).await?;
    let session = select(&catalog.terminals, &selector)?;
    let fields = session.fields();
    let command = HubCommand::Terminal {
        command: HubTerminalCommand {
            request_id: uuid::Uuid::new_v4(),
            selection: TerminalSelection {
                address: fields.address,
                expected_runtime: fields.runtime.map(|runtime| runtime.target.runtime_epoch),
            },
            action,
        },
    };
    let deadline = tokio::time::Instant::now() + command.operation_timeout(CATALOG_TIMEOUT);
    let reply = client
        .call(fields.address.hub_id, command, deadline)
        .await?;
    let Reply::Terminal { terminal } = reply else {
        anyhow::bail!(UNEXPECTED_LIFECYCLE_REPLY);
    };
    let terminal = if matches!(action, TerminalAction::Terminate { .. }) {
        settle_terminated(&mut client, terminal).await?
    } else {
        terminal
    };
    let mut output = io::stdout().lock();
    if json {
        serde_json::to_writer_pretty(&mut output, &terminal)?;
        writeln!(output)?;
    } else {
        let fields = terminal.fields();
        writeln!(
            output,
            "{}\t{}\t{}",
            fields.address,
            crate::listing::state(&terminal),
            fields.name
        )?;
    }
    Ok(())
}

/// A terminate reply can precede the catalogue's settled exit row; report the
/// settled summary, or the last observation if it does not settle in time.
async fn settle_terminated<S>(
    client: &mut yolo_hub_client::Client<S>,
    mut terminal: TerminalSummary,
) -> Result<TerminalSummary>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let deadline = tokio::time::Instant::now() + LIFECYCLE_SETTLE_TIMEOUT;
    while !settled(&terminal) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(LIFECYCLE_SETTLE_POLL).await;
        let Ok(catalog) =
            yolo_hub_catalog_client::read_with_archived(client, deadline, false).await
        else {
            break;
        };
        if let Some(found) = catalog
            .terminals
            .into_iter()
            .find(|candidate| candidate.fields().address == terminal.fields().address)
        {
            terminal = found;
        }
    }
    Ok(terminal)
}

pub(crate) fn settled(terminal: &TerminalSummary) -> bool {
    terminal.fields().runtime.is_none_or(|runtime| {
        !matches!(
            runtime.status,
            RuntimeStatus::Running
                | RuntimeStatus::Starting
                | RuntimeStatus::Finalizing
                | RuntimeStatus::OutputBlocked
                | RuntimeStatus::Unreachable
        )
    })
}

pub(crate) fn select<'a>(
    terminals: &'a [TerminalSummary],
    selector: &str,
) -> Result<&'a TerminalSummary> {
    let mut matches = terminals
        .iter()
        .filter(|terminal| !terminal.fields().archived && terminal.matches_selector(selector));
    let selected = matches.next().context(NO_MATCHING_TERMINAL)?;
    ensure!(matches.next().is_none(), AMBIGUOUS_TERMINAL);
    Ok(selected)
}
