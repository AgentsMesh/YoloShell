use crate::constants::*;
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use tokio::process::Command;
use yolo_hub_identity::HubId;

pub(crate) struct RemoteIdentity {
    pub(crate) hub: HubId,
    pub(crate) public_key: String,
    pub(crate) peer_socket: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GrantReceipt {
    Applied,
    Pending,
    Unknown,
}

pub(crate) async fn identity(target: &str, remote_yolo: &str) -> Result<RemoteIdentity> {
    let output = Command::new(SSH_PROGRAM)
        .arg("-T")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg(target)
        .arg(identity_command(remote_yolo))
        .output()
        .await
        .context(MACHINE_SSH_FAILED)?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let hint = if stderr.contains("unrecognized subcommand") {
        format!("; {MACHINE_OLDER_PROVIDER_HINT}")
    } else {
        String::new()
    };
    ensure!(
        output.status.success(),
        "{}: {}{hint}",
        MACHINE_SSH_FAILED,
        stderr.trim()
    );
    parse_identity(&String::from_utf8_lossy(&output.stdout))
}

pub(crate) async fn grant_remote(
    target: &str,
    remote_yolo: &str,
    consumer: HubId,
    identity: yolo_hub_delegation::PeerIdentity,
) -> Result<GrantReceipt> {
    let output = match Command::new(SSH_PROGRAM)
        .arg("-T")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg(target)
        .arg(grant_command(
            remote_yolo,
            consumer,
            crate::output::key_text(identity),
        ))
        .output()
        .await
    {
        Ok(output) => output,
        Err(_) => return Ok(GrantReceipt::Unknown),
    };
    if !output.status.success() {
        return Ok(GrantReceipt::Unknown);
    }
    Ok(if parse_applied(&String::from_utf8_lossy(&output.stdout))? {
        GrantReceipt::Applied
    } else {
        GrantReceipt::Pending
    })
}

fn identity_command(remote_yolo: &str) -> String {
    if remote_yolo == REMOTE_YOLO_DEFAULT {
        "if command -v yolo >/dev/null 2>&1; then exec yolo machine identity --json; else exec \
         \"$HOME/.local/bin/yolo\" machine identity --json; fi"
            .into()
    } else {
        format!("exec {} machine identity --json", shell_quote(remote_yolo))
    }
}

fn grant_command(remote_yolo: &str, consumer: HubId, public_key: String) -> String {
    let command = format!(
        "machine grant --consumer {} --public-key {} --json",
        shell_quote(&consumer.to_string()),
        shell_quote(&public_key),
    );
    if remote_yolo == REMOTE_YOLO_DEFAULT {
        format!(
            "if command -v yolo >/dev/null 2>&1; then exec yolo {command}; else exec \"$HOME/.local/bin/yolo\" {command}; fi"
        )
    } else {
        format!("exec {} {command}", shell_quote(remote_yolo))
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
            hub: hub.parse().context(MACHINE_INVALID_RESPONSE)?,
            public_key: public_key.to_owned(),
            peer_socket: peer_socket.into(),
        });
    }
    anyhow::bail!(MACHINE_INVALID_RESPONSE)
}

fn parse_applied(stdout: &str) -> Result<bool> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout.trim())
        && let Some(applied) = value.get("applied").and_then(|value| value.as_bool())
    {
        return Ok(applied);
    }
    for line in stdout.lines().rev() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        if let Some(applied) = value.get("applied").and_then(|value| value.as_bool()) {
            return Ok(applied);
        }
    }
    anyhow::bail!(MACHINE_REMOTE_GRANT_FAILED)
}

#[cfg(test)]
mod tests {
    use super::parse_applied;

    #[test]
    fn parses_pretty_remote_receipts() {
        assert!(parse_applied("{\n  \"applied\": true\n}").unwrap());
        assert!(!parse_applied("{\n  \"applied\": false\n}").unwrap());
    }
}
