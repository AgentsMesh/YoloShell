use crate::constants::*;
use crate::output;
use crate::JoinPeer;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_enrollment_protocol::decode;
use yolo_hub_enrollment_wire::{JOIN_SCHEMA_VERSION, JoinErrorCode, JoinRequest};
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_peer_transport::JoinFailure;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) async fn join(
    socket: &Path,
    entry: HubId,
    args: &JoinPeer,
    deadline: Instant,
) -> Result<()> {
    let code = args.code.trim();
    let invite = decode(code).context(INVALID_INVITE_CODE)?;
    invite
        .verify_at(super::wall_clock_ms()?)
        .context(INVALID_INVITE_CODE)?;
    let dial = crate::join_dial(&invite, args)?;
    let snapshot = super::read(socket, entry, deadline).await?;
    let local = snapshot.state.identity;
    ensure!(
        invite.claims().issuer.hub() != local.hub() && invite.claims().issuer.key() != local.key(),
        PEER_IS_LOCAL
    );
    // Reject a local configuration conflict before the other Hub admits us.
    let change = crate::join_change(&snapshot, invite.claims(), &dial)?;
    let request = JoinRequest {
        schema_version: JOIN_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        invite_id: invite.claims().invite_id,
        peer: local,
    };
    let reply = yolo_hub_peer_transport::join(
        &dial.host,
        dial.port,
        code,
        &invite,
        &request,
        deadline,
    )
    .await
    .map_err(|failure| match failure {
        JoinFailure::Connect => anyhow::anyhow!("{JOIN_CONNECT_FAILED}"),
        JoinFailure::HostKeyMismatch => anyhow::anyhow!("{JOIN_HOST_KEY_MISMATCH}"),
        JoinFailure::Authentication => anyhow::anyhow!("{JOIN_AUTHENTICATION_FAILED}"),
        JoinFailure::Exchange => anyhow::anyhow!("{JOIN_EXCHANGE_FAILED}"),
        JoinFailure::Rejected(JoinErrorCode::Unavailable) => {
            anyhow::anyhow!("{ENROLLMENT_FAILED}: {ENROLLMENT_UNAVAILABLE}")
        }
        JoinFailure::Rejected(code) => anyhow::anyhow!("{ENROLLMENT_FAILED}: {code}"),
    })?;
    ensure!(reply.hub == invite.claims().issuer, ENROLLMENT_FAILED);
    let changed = change.is_some();
    let updated = match change {
        Some(change) => execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
            .await
            .context(HUB_CONFIGURATION_WRITE_FAILED)?,
        None => snapshot,
    };
    if args.json {
        println!(
            "{}",
            output::join_json(
                &updated,
                &dial,
                &reply,
                changed,
                invite.claims().expires_at_ms,
            )?
        );
    } else {
        println!(
            "{JOINED} {} at {}:{} (revision {})",
            reply.hub.hub(),
            dial.host,
            dial.port,
            updated.state.revision
        );
        if !reply.applied {
            println!("{JOIN_PERSISTED_NOT_APPLIED}");
        }
    }
    Ok(())
}
