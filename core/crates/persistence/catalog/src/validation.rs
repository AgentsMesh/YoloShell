use super::constants::{
    ARCHIVED_COLUMN, ARCHIVED_FALSE, MANIFEST_JSON_COLUMN, SESSION_ID_COLUMN, UPDATED_AT_COLUMN,
    archived_value,
};
use super::*;
use rusqlite::params_from_iter;

const RAW_ALL_QUERY: &str = "SELECT session_id, manifest_json, updated_at_ms, archived \
    FROM sessions ORDER BY updated_at_ms DESC, session_id";
const RAW_ACTIVE_QUERY: &str = "SELECT session_id, manifest_json, updated_at_ms, archived \
    FROM sessions WHERE archived = ?1 ORDER BY updated_at_ms DESC, session_id";
#[derive(Debug, PartialEq, Eq)]
struct CanonicalRow {
    terminal_id: String,
    manifest_json: Vec<u8>,
    updated_at_ms: i64,
    archived: i64,
}

pub(super) fn validate_durable_snapshot(
    catalog: &Catalog,
    durable: &[TerminalManifest],
) -> Result<()> {
    let expected = expected_rows(durable)?;
    catalog_content(
        read_raw_rows(catalog, RAW_ALL_QUERY, None)? == expected,
        "catalog rows differ from durable manifests",
    )?;
    let expected_active: Vec<_> = expected
        .into_iter()
        .filter(|row| row.archived == ARCHIVED_FALSE)
        .collect();
    catalog_content(
        read_raw_rows(catalog, RAW_ACTIVE_QUERY, Some(ARCHIVED_FALSE))? == expected_active,
        "active catalog rows differ from durable manifests",
    )?;
    for manifest in durable {
        let indexed = catalog.get(manifest.terminal_id)?;
        let indexed_json = indexed.as_ref().map(serde_json::to_vec).transpose()?;
        catalog_content(
            indexed_json.as_deref() == Some(serde_json::to_vec(manifest)?.as_slice()),
            format!(
                "canonical catalog lookup differs for {}",
                manifest.terminal_id
            ),
        )?;
    }
    Ok(())
}

fn expected_rows(manifests: &[TerminalManifest]) -> Result<Vec<CanonicalRow>> {
    let mut rows = Vec::with_capacity(manifests.len());
    for manifest in manifests {
        validate_session_manifest(manifest)?;
        let terminal_id = manifest.terminal_id.to_string();
        anyhow::ensure!(
            !rows
                .iter()
                .any(|row: &CanonicalRow| row.terminal_id == terminal_id),
            "duplicate durable manifest {}",
            manifest.terminal_id
        );
        rows.push(CanonicalRow {
            terminal_id,
            manifest_json: serde_json::to_vec(manifest)?,
            updated_at_ms: manifest.updated_at_ms,
            archived: archived_value(manifest.archived),
        });
    }
    rows.sort_by(|left, right| {
        right
            .updated_at_ms
            .cmp(&left.updated_at_ms)
            .then_with(|| left.terminal_id.cmp(&right.terminal_id))
    });
    Ok(rows)
}

fn read_raw_rows(
    catalog: &Catalog,
    query: &str,
    archived: Option<i64>,
) -> Result<Vec<CanonicalRow>> {
    let mut statement = super::rebuild_reason::schema_result(catalog.connection.prepare(query))?;
    let rows = super::rebuild_reason::schema_result(statement.query_map(
        params_from_iter(archived),
        |row| {
            Ok(CanonicalRow {
                terminal_id: row.get(SESSION_ID_COLUMN)?,
                manifest_json: row.get(MANIFEST_JSON_COLUMN)?,
                updated_at_ms: row.get(UPDATED_AT_COLUMN)?,
                archived: row.get(ARCHIVED_COLUMN)?,
            })
        },
    ))?;
    let mut canonical = Vec::new();
    for row in rows {
        let row = super::rebuild_reason::schema_result(row)?;
        let manifest: TerminalManifest = serde_json::from_slice(&row.manifest_json)
            .map_err(|error| super::rebuild_reason::content(error.to_string()))?;
        validate_session_manifest(&manifest)
            .map_err(|error| super::rebuild_reason::content(error.to_string()))?;
        catalog_content(
            row.terminal_id == manifest.terminal_id.to_string(),
            "catalog row identity is not canonical",
        )?;
        catalog_content(
            row.manifest_json == serde_json::to_vec(&manifest)?,
            "catalog manifest JSON is not canonical",
        )?;
        catalog_content(
            row.updated_at_ms == manifest.updated_at_ms,
            "catalog update timestamp differs from manifest",
        )?;
        catalog_content(
            row.archived == archived_value(manifest.archived),
            "catalog archive flag differs from manifest",
        )?;
        canonical.push(row);
    }
    Ok(canonical)
}

fn catalog_content(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(super::rebuild_reason::content(message))
    }
}
