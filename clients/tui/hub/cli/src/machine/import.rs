use crate::AddMachine;
use crate::configuration::{identity, import_change};
use crate::constants::*;
use crate::machine::remote::{GrantReceipt, grant_remote, identity as remote_identity};
use anyhow::{Context, Result, ensure};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_federation_settings::validate_ssh_target;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::{execute, route_capabilities};
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) async fn observe(
    socket: &Path,
    entry: HubId,
    args: &AddMachine,
    deadline: Instant,
) -> Result<()> {
    run(socket, entry, args, deadline, false).await
}

pub(crate) async fn connect(
    socket: &Path,
    entry: HubId,
    args: &AddMachine,
    deadline: Instant,
) -> Result<()> {
    ensure!(!args.pin_only, "machine connect cannot use --pin-only");
    run(socket, entry, args, deadline, true).await
}

async fn run(
    socket: &Path,
    entry: HubId,
    args: &AddMachine,
    deadline: Instant,
    control: bool,
) -> Result<()> {
    validate_ssh_target(&args.target)?;
    let remote = remote_identity(&args.target, &args.remote_yolo).await?;
    let peer = identity(remote.hub, &remote.public_key)?;
    let snapshot = super::read(socket, entry, deadline).await?;
    let change = import_change(
        &snapshot,
        args.target.clone(),
        remote.peer_socket,
        peer,
        args.pin_only,
    )?;
    let applied = match change {
        Some(change) => {
            let desired = change.configuration.clone();
            match execute(socket, entry, HubRpcCommand::Configure { change }, deadline).await {
                Ok(snapshot) => snapshot.applied,
                Err(yolo_hub_management_client::ManagementError::OutcomeUnknown) => {
                    let snapshot = super::read(socket, entry, deadline).await?;
                    ensure!(
                        snapshot.state.configuration == desired,
                        HUB_CONFIGURATION_WRITE_FAILED
                    );
                    snapshot.applied
                }
                Err(error) => return Err(error).context(HUB_CONFIGURATION_WRITE_FAILED),
            }
        }
        None => snapshot.applied,
    };
    let mut remote_receipt = None;
    if control && !args.pin_only && applied {
        remote_receipt = Some(grant_remote(
            &args.target,
            &args.remote_yolo,
            entry,
            snapshot.state.identity,
        )
        .await
        .context(MACHINE_REMOTE_GRANT_FAILED)?);
    }
    let route_ready = if matches!(remote_receipt, Some(GrantReceipt::Applied)) {
        route_capabilities(socket, entry, peer.hub(), deadline)
            .await
            .is_ok_and(|capabilities| {
                capabilities.lifecycle
                    && capabilities.terminal_read
                    && capabilities.terminal_write
                    && capabilities.agent_start
            })
    } else {
        false
    };
    let control_applied = control
        && !args.pin_only
        && applied
        && matches!(remote_receipt, Some(GrantReceipt::Applied))
        && route_ready;
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "machine": peer.hub().to_string(),
                "ssh": args.target,
                "imports": !args.pin_only,
                "control": control_applied,
                "control_pending": control && !args.pin_only && !control_applied,
                "applied": applied,
                "route_ready": route_ready,
            })
        );
    } else if control && control_applied {
        println!(
            "connected {} via {} with control access",
            peer.hub(),
            args.target
        );
        if !applied {
            println!("local configuration is persisted but not active");
        }
    } else if control {
        println!(
            "connected {} via {} with control access pending",
            peer.hub(),
            args.target
        );
        println!("the local or remote service has not applied the control policy yet");
        if !applied {
            println!("local configuration is persisted but not active");
        }
    } else if args.pin_only {
        println!("pinned machine {} via {}", peer.hub(), args.target);
        if !applied {
            println!("configuration is persisted but not active");
        }
    } else {
        println!(
            "observing {} via {} ({IMPORT_ONE_WAY})",
            peer.hub(),
            args.target
        );
        if !applied {
            println!("configuration is persisted but not active");
        }
    }
    Ok(())
}
