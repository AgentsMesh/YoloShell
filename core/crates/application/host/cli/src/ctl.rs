use super::{CtlCommand, CtlOptions};
use anyhow::Result;
use yolo_client::Client;
use yolo_runtime::model::{WriterCapability, WriterLeaseGrant};
use yolo_runtime::worker::RuntimeRef;

#[path = "ctl_client.rs"]
mod ctl_client;
#[path = "ctl_lifecycle.rs"]
mod ctl_lifecycle;
#[path = "ctl_presence.rs"]
mod ctl_presence;
#[path = "ctl_runtime.rs"]
mod ctl_runtime;
use ctl_runtime::runtime_target;
pub(super) use ctl_runtime::validate_mutation_authorization;

pub(super) async fn run(options: CtlOptions) -> Result<()> {
    if let Some(absence) = ctl_presence::absence(&options)? {
        return ctl_presence::write_absence(absence, std::io::stdout().lock());
    }
    let client = ctl_client::open(&options).await?;
    match options.command {
        CtlCommand::Status => print_native(&client.status().await?)?,
        CtlCommand::ShutdownIfIdle => { client.shutdown_if_idle().await?; print_ok()?; },
        CtlCommand::List {
            include_archived,
            require_complete,
        } => {
            let sessions = if require_complete {
                client.list_sessions_complete(include_archived).await?
            } else {
                client.list_sessions(include_archived).await?
            };
            print_native(&sessions)?
        }
        CtlCommand::Create {
            authorization,
            export_creation_authorization,
            name,
            cwd,
        } => {
            create_session(
                &client,
                authorization,
                export_creation_authorization,
                name,
                cwd,
            )
            .await?
        }
        CtlCommand::Get { terminal_id } => print_native(&client.get_session(terminal_id).await?)?,
        CtlCommand::Snapshot { terminal_id } => print_native(&client.snapshot(terminal_id).await?)?,
        CtlCommand::Attach {
            terminal_id,
            after_seq,
            events,
        } => {
            let mut attachment = client.attach(terminal_id, after_seq).await?;
            print_native_frame(&attachment.initial)?;
            for _ in 0..events {
                print_native_frame(&attachment.next_event().await?)?;
            }
        }
        CtlCommand::Acquire {
            terminal_id,
            holder,
            export_writer_authorization,
        } => {
            let target = runtime_target(&client, terminal_id).await?;
            let grant = client.acquire_writer(target, holder, None).await?;
            print_writer_grant(target, &grant, export_writer_authorization)?;
        }
        CtlCommand::Takeover {
            terminal_id,
            holder,
            observed_generation,
            export_writer_authorization,
        } => {
            let target = runtime_target(&client, terminal_id).await?;
            let grant = client
                .take_over_writer_for_instance(
                    target,
                    holder,
                    None,
                    Some(observed_generation),
                    WriterCapability::generate(),
                    None,
                )
                .await?;
            print_writer_grant(target, &grant, export_writer_authorization)?;
        }
        CtlCommand::Input {
            terminal_id,
            authorization,
            data,
        } => {
            let authorization = validate_mutation_authorization(terminal_id, authorization.read()?)?;
            print_native(
                &client
                    .input(
                        authorization.target,
                        authorization.grant.generation,
                        &authorization.grant.writer_capability,
                        data.into_bytes(),
                    )
                    .await?,
            )?;
        }
        CtlCommand::Resize {
            terminal_id,
            authorization,
            cols,
            rows,
        } => {
            let authorization = validate_mutation_authorization(terminal_id, authorization.read()?)?;
            client
                .resize(
                    authorization.target,
                    authorization.grant.generation,
                    &authorization.grant.writer_capability,
                    cols,
                    rows,
                )
                .await?;
            print_ok()?;
        }
        CtlCommand::Signal {
            terminal_id,
            authorization,
            signal,
        } => {
            let authorization = validate_mutation_authorization(terminal_id, authorization.read()?)?;
            client
                .signal(
                    authorization.target,
                    authorization.grant.generation,
                    &authorization.grant.writer_capability,
                    signal,
                )
                .await?;
            print_ok()?;
        }
        CtlCommand::Terminate {
            terminal_id,
            force,
            preconditions,
        } => {
            ctl_lifecycle::terminate(&client, terminal_id, force, preconditions).await?;
            print_ok()?;
        }
        CtlCommand::Archive {
            terminal_id,
            preconditions,
        } => print_native(&ctl_lifecycle::archive(&client, terminal_id, preconditions).await?)?,
        CtlCommand::Discard {
            terminal_id,
            force,
            preconditions,
        } => print_native(&ctl_lifecycle::discard(&client, terminal_id, force, preconditions).await?)?,
        CtlCommand::CleanupTerminate {
            terminal_id,
            authorization,
            force,
        } => {
            print_native(&cleanup_terminate(&client, authorization, terminal_id, force).await?)?;
        }
        CtlCommand::CleanupArchive {
            terminal_id,
            authorization,
        } => print_native(&cleanup_archive(&client, authorization, terminal_id).await?)?,
        CtlCommand::CleanupDiscard {
            terminal_id,
            authorization,
            force,
        } => print_native(&cleanup_discard(&client, authorization, terminal_id, force).await?)?,
        CtlCommand::FinalizeCreation {
            terminal_id,
            authorization,
        } => {
            finalize_creation(&client, authorization, terminal_id).await?;
            print_ok()?;
        }
        CtlCommand::Resume {
            terminal_id,
            recreate,
            preconditions,
        } => {
            print_native(&ctl_lifecycle::resume(&client, terminal_id, recreate, preconditions).await?)?
        }
        CtlCommand::Reconcile => print_native(&client.reconcile().await?)?,
    }
    Ok(())
}

include!("ctl_output.rs");
include!("ctl_creation.rs");
