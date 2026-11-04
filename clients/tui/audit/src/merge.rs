use crate::print_with_source;
use crate::constants::*;
use anyhow::{Result, ensure};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use yolo_hub_audit::{AuditEntry, AuditEntryDocument, AuditTrust, read};
use yolo_hub_identity::HubId;

pub(crate) struct Merged {
    pub sources: Vec<serde_json::Value>,
    pub duplicates: u64,
    pub conflicts: u64,
    pub authenticated: u64,
    pub unsigned: u64,
    pub invalid: u64,
    pub untrusted: u64,
    pub records: Vec<(AuditEntry, String)>,
}

pub(crate) fn run(paths: &[PathBuf], limit: u16, json: bool, trust: &[String]) -> Result<()> {
    let trust = (!trust.is_empty())
        .then(|| AuditTrust::parse(trust).map_err(|error| anyhow::anyhow!(error.to_string())))
        .transpose()?;
    let mut merged = merge(paths, trust.as_ref())?;
    if trust.is_some() {
        ensure!(
            merged.untrusted == 0 && merged.unsigned == 0 && merged.invalid == 0,
            "{AUDIT_MERGE_UNAUTHENTICATED}: unsigned={} invalid={} untrusted={}",
            merged.unsigned,
            merged.invalid,
            merged.untrusted
        );
    }
    merged.records.truncate(usize::from(limit));
    if json {
        let rows = merged
            .records
            .iter()
            .map(|(entry, source)| {
                let mut value = serde_json::to_value(AuditEntryDocument::from(entry))?;
                value[AUDIT_SOURCE_FIELD] = json!(source);
                Ok(value)
            })
            .collect::<Result<Vec<_>>>()?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "sources": merged.sources,
                "duplicates": merged.duplicates,
                "conflicts": merged.conflicts,
                "authenticated": merged.authenticated,
                "unsigned": merged.unsigned,
                "invalid": merged.invalid,
                "records": rows,
            }))?
        );
        return Ok(());
    }
    for (entry, source) in &merged.records {
        print_with_source(source, entry);
    }
    Ok(())
}

pub(crate) fn merge(paths: &[PathBuf], trust: Option<&AuditTrust>) -> Result<Merged> {
    ensure!(
        paths.len() <= AUDIT_MERGE_MAX_SOURCES,
        "at most {AUDIT_MERGE_MAX_SOURCES} audit sources can be merged"
    );
    let mut seen: BTreeMap<(HubId, u64), (AuditEntry, String)> = BTreeMap::new();
    let mut sources = Vec::new();
    let mut duplicates = 0_u64;
    let mut conflicts = 0_u64;
    let mut authenticated = 0_u64;
    let mut unsigned = 0_u64;
    let mut invalid = 0_u64;
    let mut untrusted = 0_u64;
    for path in paths {
        let source = source_path(path)?;
        let entries = read(path)?;
        sources.push(json!({"path": source, "records": entries.len()}));
        for entry in entries {
            if entry.authenticated() {
                authenticated += 1;
            } else if entry.signer.is_some() {
                invalid += 1;
            } else {
                unsigned += 1;
            }
            if trust.is_some_and(|trust| !trust.accepts(&entry)) {
                untrusted += 1;
            }
            let key = (entry.record.destination, entry.record.sequence);
            match seen.get(&key) {
                None => {
                    seen.insert(key, (entry, source.clone()));
                }
                Some((existing, _)) if existing.record == entry.record => duplicates += 1,
                Some(_) => conflicts += 1,
            }
        }
    }
    let mut records = seen.into_values().collect::<Vec<_>>();
    records.sort_by_key(|(entry, _)| {
        (
            entry.record.at_ms,
            entry.record.destination,
            entry.record.sequence,
        )
    });
    Ok(Merged {
        sources,
        duplicates,
        conflicts,
        authenticated,
        unsigned,
        invalid,
        untrusted,
        records,
    })
}

fn source_path(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)?;
    ensure!(metadata.is_file(), AUDIT_MERGE_FILE);
    ensure!(
        metadata.len() <= AUDIT_MERGE_MAX_BYTES,
        "audit source exceeds the {AUDIT_MERGE_MAX_BYTES}-byte limit"
    );
    Ok(path.to_string_lossy().into_owned())
}
