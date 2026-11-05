use crate::configuration::{identity, join_change};
use crate::constants::*;
use crate::JoinPeer;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::time::Instant;
use yolo_hub_federation_settings::validate_ssh_target;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

struct RemoteIdentity {
    hub: HubId,
    public_key: String,
    peer_socket: PathBuf,
}

pub(crate) async fn run(
    socket: &Path,
    entry: HubId,
    args: &JoinPeer,
    deadline: Instant,
) -> Result<()> {
    validate_ssh_target(&args.target)?;
    let remote = remote_identity(&args.target, &args.remote_yolo).await?;
    let peer = identity(remote.hub, &remote.public_key)?;
    let snapshot = super::read(socket, entry, deadline).await?;
    let change = join_change(
        &snapshot,
        args.target.clone(),
        remote.peer_socket,
        peer,
        args.no_import,
    )?;
    let applied = match change {
        Some(change) => {
            execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
                .await
                .context(HUB_CONFIGURATION_WRITE_FAILED)?
                .applied
        }
        None => snapshot.applied,
    };
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "hub": peer.hub().to_string(),
                "ssh": args.target,
                "consume": !args.no_import,
                "applied": applied,
            })
        );
    } else {
        println!("joined {} via {}", peer.hub(), args.target);
        if !applied {
            println!("configuration is persisted but not active");
        }
    }
    Ok(())
}

async fn remote_identity(target: &str, remote_yolo: &str) -> Result<RemoteIdentity> {
    let output = Command::new(SSH_PROGRAM)
        .arg("-T")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg(target)
        .arg(remote_command(remote_yolo))
        .output()
        .await
        .context(JOIN_SSH_FAILED)?;
    ensure!(
        output.status.success(),
        "{}: {}",
        JOIN_SSH_FAILED,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    parse_identity(&String::from_utf8_lossy(&output.stdout))
}

fn remote_command(remote_yolo: &str) -> String {
    if remote_yolo == JOIN_REMOTE_YOLO {
        "if command -v yolo >/dev/null 2>&1; then exec yolo hub identity --json; else exec \
         \"$HOME/.local/bin/yolo\" hub identity --json; fi"
            .into()
    } else {
        format!("exec {} hub identity --json", shell_quote(remote_yolo))
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn parse_identity(stdout: &str) -> Result<RemoteIdentity> {
    for line in stdout.lines().rev() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        let Some(hub) = value.get(HUB_ID_FIELD).and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(public_key) = value.get(PUBLIC_KEY_FIELD).and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(peer_socket) = value.get("peer_socket").and_then(|value| value.as_str()) else {
            continue;
        };
        return Ok(RemoteIdentity {
            hub: hub.parse().context(JOIN_INVALID_RESPONSE)?,
            public_key: public_key.to_owned(),
            peer_socket: peer_socket.into(),
        });
    }
    anyhow::bail!(JOIN_INVALID_RESPONSE)
}
