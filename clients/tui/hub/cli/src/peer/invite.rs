use crate::constants::*;
use crate::output;
use crate::InvitePeer;
use anyhow::{Context, Result, ensure};
use std::net::SocketAddr;
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute_reply;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::{Command as HubRpcCommand, Reply};

/// Minting an invite is a request to accept a peer. Provision the documented
/// default listener so the joining side can dial it without a second command.
pub(crate) fn provisioned_listener(
    snapshot: &ConfigurationSnapshot,
) -> Result<Option<SocketAddr>> {
    if snapshot.listen_address.is_some() {
        return Ok(None);
    }
    if let Some(configured) = snapshot.state.configuration.listen {
        anyhow::bail!("{INVITE_LISTENER_NOT_APPLIED} (configured: {configured})");
    }
    Ok(Some(SocketAddr::from(([0, 0, 0, 0], DEFAULT_PEER_PORT))))
}

pub(crate) async fn invite(
    socket: &Path,
    entry: HubId,
    args: &InvitePeer,
    deadline: Instant,
) -> Result<()> {
    let mut snapshot = super::read(socket, entry, deadline).await?;
    if let Some(address) = provisioned_listener(&snapshot)? {
        let change = crate::listen_change(&snapshot, address)?;
        snapshot = yolo_hub_management_client::execute(
            socket,
            entry,
            HubRpcCommand::Configure { change },
            deadline,
        )
        .await
        .context(HUB_CONFIGURATION_WRITE_FAILED)?;
        ensure!(
            snapshot.applied,
            "{INVITE_LISTENER_NOT_APPLIED} (requested: {address})"
        );
        eprintln!("configured peer listener {address}");
    }
    let request = crate::invite_request(args)?;
    let reply = execute_reply(
        socket,
        entry,
        HubRpcCommand::MintInvite { request },
        deadline,
    )
    .await
    .context(INVITE_MINT_FAILED)?;
    let Reply::Minted { invite } = reply else {
        anyhow::bail!(INVITE_MINT_FAILED)
    };
    if args.json {
        println!("{}", output::invite_json(&snapshot, &invite)?);
    } else {
        println!("{INVITE_CODE_HEADER} ({} min):", args.ttl_minutes);
        println!("{}", invite.code);
        println!(
            "{INVITE_EXPIRES}: {}",
            expires_in_text(super::wall_clock_ms()?, invite.expires_at_ms)
        );
        println!("{INVITE_JOIN_HINT}");
    }
    Ok(())
}

pub(crate) fn expires_in_text(now_ms: u64, expires_at_ms: u64) -> String {
    let seconds = expires_at_ms.saturating_sub(now_ms) / 1000;
    match seconds {
        0 => "less than a minute".into(),
        seconds if seconds < 60 => format!("{seconds}s"),
        seconds => format!("{}m {:02}s", seconds / 60, seconds % 60),
    }
}
