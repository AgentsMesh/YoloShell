use crate::constants::*;
use crate::output;
use crate::{AddPeer, InvitePeer, JoinPeer};
use anyhow::{Context, Result, ensure};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_enrollment_protocol::decode;
use yolo_hub_enrollment_wire::{JOIN_SCHEMA_VERSION, JoinRequest};
use yolo_hub_identity::HubId;
use yolo_hub_management_client::{execute, execute_reply};
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::{Command as HubRpcCommand, Reply};

pub(crate) async fn add(
    socket: &Path,
    entry: HubId,
    args: &AddPeer,
    deadline: Instant,
) -> Result<()> {
    let snapshot = read(socket, entry, deadline).await?;
    let change = crate::add_change(&snapshot, args)?;
    let updated = execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
        .await
        .context(HUB_CONFIGURATION_WRITE_FAILED)?;
    if args.json {
        println!("{}", output::add_json(&updated, args.peer)?);
    } else {
        println!("added peer {} at revision {}", args.peer, updated.state.revision);
        if !updated.applied {
            println!("configuration is persisted but not active");
        }
    }
    Ok(())
}

pub(crate) async fn list(socket: &Path, entry: HubId, json: bool, deadline: Instant) -> Result<()> {
    let snapshot = read(socket, entry, deadline).await?;
    if json {
        println!("{}", output::list_json(&snapshot)?);
        return Ok(());
    }
    println!("{HUB_PEER_HEADER}");
    for peer in &snapshot.state.configuration.peers {
        let dial = peer
            .dial
            .as_ref()
            .map(|dial| format!("{}:{}", dial.host, dial.port))
            .unwrap_or_else(|| "-".into());
        let imports = if peer.consume { "yes" } else { "no" };
        let exports = peer
            .exports
            .iter()
            .map(|policy| policy.origin.to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{}\t{}\t{}\t{}",
            peer.identity.hub(),
            dial,
            imports,
            if exports.is_empty() { "-".into() } else { exports }
        );
    }
    Ok(())
}

pub(crate) async fn join(
    socket: &Path,
    entry: HubId,
    args: &JoinPeer,
    deadline: Instant,
) -> Result<()> {
    let code = args.code.trim();
    let invite = decode(code).context(INVALID_INVITE_CODE)?;
    invite
        .verify_at(wall_clock_ms()?)
        .context(INVALID_INVITE_CODE)?;
    let snapshot = read(socket, entry, deadline).await?;
    let local = snapshot.state.identity;
    ensure!(
        invite.claims().issuer.hub() != local.hub()
            && invite.claims().issuer.key() != local.key(),
        PEER_IS_LOCAL
    );
    // Reject a local configuration conflict before the other Hub admits us.
    let change = crate::join_change(&snapshot, invite.claims())?;
    let request = JoinRequest {
        schema_version: JOIN_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        invite_id: invite.claims().invite_id,
        peer: local,
    };
    let reply = yolo_hub_peer_transport::join(
        &invite.claims().dial.host,
        invite.claims().dial.port,
        code,
        &invite,
        &request,
        deadline,
    )
    .await
    .map_err(|code| anyhow::anyhow!("{code}"))
    .context(ENROLLMENT_FAILED)?;
    ensure!(reply.hub == invite.claims().issuer, ENROLLMENT_FAILED);
    let changed = change.is_some();
    let updated = match change {
        Some(change) => {
            execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
                .await
                .context(HUB_CONFIGURATION_WRITE_FAILED)?
        }
        None => snapshot,
    };
    if args.json {
        println!("{}", output::join_json(&updated, &invite, &reply, changed)?);
    } else {
        println!(
            "{JOINED} {} at {}:{} (revision {})",
            reply.hub.hub(),
            invite.claims().dial.host,
            invite.claims().dial.port,
            updated.state.revision
        );
        if !reply.applied {
            println!("{JOIN_PERSISTED_NOT_APPLIED}");
        }
    }
    Ok(())
}

fn wall_clock_ms() -> Result<u64> {
    Ok(u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?)
}

pub(crate) async fn invite(
    socket: &Path,
    entry: HubId,
    args: &InvitePeer,
    deadline: Instant,
) -> Result<()> {
    let snapshot = read(socket, entry, deadline).await?;
    let request = crate::invite_request(&snapshot, args)?;
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
        println!("{INVITE_EXPIRES}: {}", invite.expires_at_ms);
        println!("{INVITE_JOIN_HINT}");
    }
    Ok(())
}

async fn read(socket: &Path, entry: HubId, deadline: Instant) -> Result<ConfigurationSnapshot> {
    execute(socket, entry, HubRpcCommand::GetConfiguration {}, deadline)
        .await
        .context(HUB_CONFIGURATION_READ_FAILED)
}
