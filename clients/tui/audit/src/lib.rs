//! Audit CLI operations: read the local signed log, merge exported files, or
//! pull signed lines from a peer through the local entry Hub.

mod constants;
pub(crate) mod merge;
mod pull;
#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use yolo_hub_audit::{AUDIT_FILE, AuditEntry, AuditEntryDocument, read, signer_document};
use yolo_hub_identity::HubId;
use yolo_local_profile::LocalProfile;

pub use constants::*;

pub struct Request<'a> {
    pub after: Option<u64>,
    pub limit: u16,
    pub json: bool,
    pub merge: &'a [PathBuf],
    pub trust: &'a [String],
    pub identity: bool,
    pub pull: Option<HubId>,
    pub output: Option<PathBuf>,
}

pub async fn run(profile: &LocalProfile, request: Request<'_>) -> Result<()> {
    ensure!(
        request.limit > 0 && request.limit <= MAX_AUDIT_LIMIT,
        "audit limit must be between 1 and {MAX_AUDIT_LIMIT}"
    );
    if request.identity {
        ensure!(
            request.merge.is_empty()
                && request.after.is_none()
                && request.pull.is_none()
                && request.output.is_none(),
            AUDIT_IDENTITY_EXCLUSIVE
        );
        return print_identity(profile, request.json);
    }
    if let Some(peer) = request.pull {
        ensure!(request.merge.is_empty(), AUDIT_IDENTITY_EXCLUSIVE);
        return pull::run(
            profile,
            peer,
            request.after,
            request.limit,
            request.output,
            request.trust,
            request.json,
        )
        .await;
    }
    if !request.merge.is_empty() {
        ensure!(request.after.is_none(), AUDIT_AFTER_WITH_MERGE);
        return merge::run(request.merge, request.limit, request.json, request.trust);
    }
    let _ = yolo_hub_state::HubState::inspect_identity(&profile.state, &profile.host_home)?
        .context(NO_LOCAL_HUB)?;
    let entries = read(&profile.state.join(AUDIT_FILE))?;
    let entries = entries
        .into_iter()
        .filter(|entry| {
            request
                .after
                .is_none_or(|after| entry.record.sequence > after)
        })
        .take(usize::from(request.limit))
        .collect::<Vec<_>>();
    if request.json {
        let documents = entries
            .iter()
            .map(AuditEntryDocument::from)
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&documents)?);
    } else {
        for entry in entries {
            print_with_source("", &entry);
        }
    }
    Ok(())
}

fn print_identity(profile: &LocalProfile, json: bool) -> Result<()> {
    let identity =
        yolo_hub_state::HubState::inspect_peer_identity(&profile.state, &profile.host_home)?
            .context(NO_LOCAL_HUB)?;
    let document = signer_document(&identity);
    if json {
        println!("{}", serde_json::to_string_pretty(&document)?);
    } else {
        println!("{}\t{}", document.hub_id, document.public_key);
    }
    Ok(())
}

pub(crate) fn print_with_source(source: &str, entry: &AuditEntry) {
    if !source.is_empty() {
        print!("{source}\t");
    }
    let record = &entry.record;
    println!(
        "{}\t{}\t{}\t{}\t{:?}\t{}\t{}\t{}",
        record.sequence,
        record.at_ms,
        record.principal,
        record.method,
        record.outcome,
        record.target.as_deref().unwrap_or("-"),
        record
            .request_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".into()),
        AuditEntryDocument::from(entry).signature.label()
    );
}
