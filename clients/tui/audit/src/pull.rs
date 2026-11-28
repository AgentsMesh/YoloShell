//! `audit --pull`: fetch a peer Hub's signed audit JSONL through the local
//! entry Hub. Lines keep their signatures so `audit --merge --trust` can
//! authenticate them offline.

use crate::constants::*;
use crate::merge;
use anyhow::{Context, Result, bail, ensure};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use tokio::time::Instant;
use yolo_hub_audit::AuditTrust;
use yolo_hub_client::LocalClient;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;

pub(crate) async fn run(
    profile: &LocalProfile,
    peer: HubId,
    after: Option<u64>,
    limit: u16,
    output: Option<PathBuf>,
    trust: &[String],
    json: bool,
) -> Result<()> {
    ensure!(output.is_some() || trust.is_empty(), AUDIT_PULL_TRUST_REQUIRES_OUTPUT);
    let local_hub = HubState::inspect_identity(&profile.state, &profile.host_home)?
        .context(NO_LOCAL_HUB)?;
    let socket = HubState::socket_path_for(&profile.state)?;
    let deadline = Instant::now() + AUDIT_PULL_TIMEOUT;
    let mut client = LocalClient::connect_local(&socket, local_hub, deadline).await?;
    let reply = client
        .call(peer, Command::ExportAudit { after, limit }, deadline)
        .await?;
    let Reply::Audit { lines } = reply else {
        bail!(AUDIT_PULL_UNEXPECTED_REPLY);
    };
    let signers = signers(&lines);
    if let Some(path) = output {
        write_lines(&path, &lines)?;
        if !trust.is_empty() {
            verify(&path, trust)?;
        }
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "peer": peer,
                    "records": lines.len(),
                    "output": path,
                    "signers": signers,
                }))?
            );
        } else {
            println!(
                "{} audit records from {} -> {}",
                lines.len(),
                peer,
                path.display()
            );
        }
        return Ok(());
    }
    for line in lines {
        println!("{}", serde_json::to_string(&line)?);
    }
    Ok(())
}

fn write_lines(path: &Path, lines: &[serde_json::Value]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .context(AUDIT_PULL_WRITE_FAILED)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    for line in lines {
        serde_json::to_writer(&mut file, line)?;
        file.write_all(b"\n")?;
    }
    file.sync_all()?;
    Ok(())
}

fn verify(path: &Path, trust: &[String]) -> Result<()> {
    let trust = AuditTrust::parse(trust).map_err(|error| anyhow::anyhow!(error.to_string()))?;
    let merged = merge::merge(&[path.to_path_buf()], Some(&trust))?;
    ensure!(
        merged.untrusted == 0 && merged.unsigned == 0 && merged.invalid == 0,
        "{AUDIT_MERGE_UNAUTHENTICATED}: unsigned={} invalid={} untrusted={}",
        merged.unsigned,
        merged.invalid,
        merged.untrusted
    );
    Ok(())
}

fn signers(lines: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut unique = std::collections::BTreeMap::new();
    for line in lines {
        let Some(signer) = line.get("signer") else { continue };
        let Some(hub) = signer.get("hub").and_then(serde_json::Value::as_str) else {
            continue;
        };
        unique.insert(hub.to_owned(), signer.clone());
    }
    unique.into_values().collect()
}
