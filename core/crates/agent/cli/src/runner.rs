use crate::arguments::{AgentCommand, Arguments, Command, Options};
use crate::connection::{self, Connection};
use crate::constants::*;
use crate::control_commands;
use crate::current;
use crate::outcome::{self, Outcome};
use crate::queries;
use crate::runs;
use crate::reject::rejected;
use crate::labels;
use crate::lifecycle;
use crate::wait_command;
use anyhow::Result;

pub async fn run(arguments: Arguments) -> Result<Outcome> {
    let Command::Agent(Options {
        socket,
        hub,
        local_profile,
        command,
    }) = arguments.command;
    let json_mode = match &command {
        AgentCommand::List(args) => args.output.json,
        AgentCommand::Runs(args) => args.output.json,
        AgentCommand::Label(args) => args.output.json,
        AgentCommand::Current(output) => output.json,
        AgentCommand::Start(args) => args.output.json,
        AgentCommand::Stop(args) => args.output.json,
        AgentCommand::Get { output, .. } => output.json,
        AgentCommand::Send(args) => args.output.json,
        AgentCommand::Ask(args) => args.output.json,
        AgentCommand::Prompt(args) => args.output.json,
        AgentCommand::Interrupt { output, .. } => output.json,
        AgentCommand::Respond { output, .. } => output.json,
        AgentCommand::Read { output, .. } => output.json,
        AgentCommand::Wait(args) => args.output.json,
    };
    let mut client = connection::connect(&Connection {
        socket,
        hub,
        local_profile,
    })
    .await?;
    let result: Result<Outcome> = async {
        match command {
            AgentCommand::List(args) => {
                queries::list(&mut client, args.all, args.output.json).await
            }
            AgentCommand::Runs(args) => {
                runs::runs(&mut client, args).await
            }
            AgentCommand::Label(args) => labels::label(&mut client, args).await,
            AgentCommand::Current(output) => {
                queries::get(&mut client, current::terminal_selector()?, output.json).await
            }
            AgentCommand::Start(args) => {
                let cwd = match args.cwd {
                    Some(cwd) => cwd,
                    None => std::env::current_dir()
                        .map_err(|_| rejected(START_CWD_UNAVAILABLE))?
                        .display()
                        .to_string(),
                };
                lifecycle::start(
                    &mut client,
                    args.provider,
                    args.name,
                    cwd,
                    args.workspace,
                    args.current,
                    args.parent,
                    args.timeout,
                    args.output.json,
                )
                .await
            }
            AgentCommand::Stop(args) => {
                lifecycle::stop(&mut client, args.run, args.force, args.output.json).await
            }
            AgentCommand::Get {
                selector,
                current,
                output,
            } => queries::get(&mut client, select(current, selector)?, output.json).await,
            AgentCommand::Send(args) => {
                let selector = select(args.current, args.selector)?;
                control_commands::send(
                    &mut client,
                    selector,
                    args.prompt,
                    args.stdin,
                    args.take_control,
                    args.output.json,
                )
                .await
            }
            AgentCommand::Ask(args) => {
                let selector = select(args.current, args.selector)?;
                control_commands::ask(
                    &mut client,
                    selector,
                    args.prompt,
                    args.stdin,
                    args.take_control,
                    args.timeout,
                    args.output.json,
                )
                .await
            }
            AgentCommand::Prompt(args) => {
                let selector = select(args.current, args.selector)?;
                control_commands::prompt(
                    &mut client,
                    selector,
                    args.prompt,
                    args.stdin,
                    args.take_control,
                    args.wait_turn,
                    args.wait_ms,
                    args.output.json,
                )
                .await
            }
            AgentCommand::Interrupt {
                selector,
                current,
                take_control,
                output,
            } => {
                let selector = select(current, selector)?;
                control_commands::interrupt(&mut client, selector, take_control, output.json).await
            }
            AgentCommand::Respond {
                selector,
                current,
                choice,
                take_control,
                output,
            } => {
                let selector = select(current, selector)?;
                control_commands::respond(
                    &mut client,
                    selector,
                    choice,
                    take_control,
                    output.json,
                )
                .await
            }
            AgentCommand::Read {
                selector,
                current,
                limit,
                output,
            } => {
                let selector = select(current, selector)?;
                queries::read(&mut client, selector, limit, output.json).await
            }
            AgentCommand::Wait(args) => {
                let selectors = if args.current {
                    if !args.selectors.is_empty() {
                        return Err(rejected(CURRENT_SELECTOR_CONFLICT));
                    }
                    vec![current::terminal_selector()?]
                } else {
                    args.selectors
                };
                wait_command::wait(
                    &mut client,
                    selectors,
                    args.runs,
                    args.until,
                    args.timeout,
                    args.output.json,
                )
                .await
            }
        }
    }
    .await;
    result.or_else(|error| outcome::reject(json_mode, error))
}

fn select(current: bool, selector: Option<String>) -> Result<String> {
    match (current, selector) {
        (true, None) => current::terminal_selector(),
        (false, Some(selector)) => Ok(selector),
        _ => Err(rejected(SELECTOR_REQUIRED)),
    }
}
