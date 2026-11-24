use super::*;
use rusqlite::{Transaction, TransactionBehavior};

const CATALOG_REPAIR_IDENTITY_MISMATCH: &str = "catalog repair session identity mismatch";

#[derive(Debug, Eq, PartialEq)]
pub enum ControllerWrite<T> {
    Applied(T),
    Fenced,
}

impl Catalog {
    pub fn insert_for_controller(
        &mut self,
        expected: u64,
        manifest: &TerminalManifest,
    ) -> Result<ControllerWrite<()>> {
        validate_session_manifest(manifest)?;
        self.write_for_controller(expected, |transaction| {
            write_manifest(transaction, constants::INSERT_SESSION_SQL, manifest)
        })
    }

    pub fn upsert_for_controller(
        &mut self,
        expected: u64,
        manifest: &TerminalManifest,
    ) -> Result<ControllerWrite<()>> {
        validate_session_manifest(manifest)?;
        self.write_for_controller(expected, |transaction| {
            write_manifest(transaction, constants::UPSERT_SESSION_SQL, manifest)
        })
    }

    pub fn repair_for_controller(
        &mut self,
        expected_generation: u64,
        expected_row: CatalogRowExpectation,
        manifest: &TerminalManifest,
    ) -> Result<ControllerWrite<bool>> {
        validate_session_manifest(manifest)?;
        anyhow::ensure!(
            expectation_session_id(&expected_row) == Some(manifest.terminal_id),
            CATALOG_REPAIR_IDENTITY_MISMATCH
        );
        self.write_for_controller(expected_generation, |transaction| {
            if !row_matches(transaction, &expected_row)? {
                return Ok(false);
            }
            if !prepare_repair_target(transaction, &expected_row, manifest)? {
                return Ok(false);
            }
            write_manifest(transaction, constants::UPSERT_SESSION_SQL, manifest)?;
            Ok(true)
        })
    }

    #[cfg(test)]
    pub(crate) fn archive_for_controller(
        &mut self,
        expected: u64,
        terminal_id: Uuid,
        archived: bool,
    ) -> Result<ControllerWrite<Option<TerminalManifest>>> {
        self.write_for_controller(expected, |transaction| {
            let Some(mut manifest) = read_manifest(transaction, terminal_id)? else {
                return Ok(None);
            };
            manifest.archived = archived;
            manifest.updated_at_ms = now_ms();
            write_manifest(transaction, constants::UPSERT_SESSION_SQL, &manifest)?;
            Ok(Some(manifest))
        })
    }
}

include!("generation_fence_commit.rs");

fn expectation_session_id(expected: &CatalogRowExpectation) -> Option<Uuid> {
    match expected {
        CatalogRowExpectation::Valid(manifest) => Some(manifest.terminal_id),
        CatalogRowExpectation::Invalid { row_key, .. } => Uuid::parse_str(row_key).ok(),
    }
}

fn write_manifest(
    connection: &Connection,
    statement: &str,
    manifest: &TerminalManifest,
) -> Result<()> {
    let encoded = serde_json::to_vec(manifest)?;
    connection.execute(
        statement,
        params![
            manifest.terminal_id.to_string(),
            encoded,
            manifest.updated_at_ms,
            constants::archived_value(manifest.archived)
        ],
    )?;
    Ok(())
}

fn row_matches(connection: &Connection, expected: &CatalogRowExpectation) -> Result<bool> {
    match expected {
        CatalogRowExpectation::Valid(manifest) => {
            let Some(current) = read_manifest(connection, manifest.terminal_id)? else {
                return Ok(false);
            };
            Ok(serde_json::to_vec(&current)? == serde_json::to_vec(manifest)?)
        }
        CatalogRowExpectation::Invalid {
            row_key,
            manifest_json,
        } => Ok(read_raw_manifest(connection, row_key)?.as_ref() == Some(manifest_json)),
    }
}

fn prepare_repair_target(
    connection: &Connection,
    expected: &CatalogRowExpectation,
    manifest: &TerminalManifest,
) -> Result<bool> {
    let CatalogRowExpectation::Invalid { row_key, .. } = expected else {
        return Ok(true);
    };
    let canonical = manifest.terminal_id.to_string();
    if row_key != &canonical && read_raw_manifest(connection, &canonical)?.is_some() {
        return Ok(false);
    }
    connection.execute(constants::DELETE_SESSION_SQL, [row_key])?;
    Ok(true)
}

fn read_raw_manifest(connection: &Connection, row_key: &str) -> Result<Option<Vec<u8>>> {
    connection
        .query_row(constants::READ_SESSION_SQL, [row_key], |row| {
            row.get(constants::MANIFEST_JSON_COLUMN)
        })
        .optional()
        .map_err(Into::into)
}

fn read_manifest(connection: &Connection, terminal_id: Uuid) -> Result<Option<TerminalManifest>> {
    let encoded: Option<Vec<u8>> = connection
        .query_row(
            constants::READ_SESSION_SQL,
            [terminal_id.to_string()],
            |row| row.get(constants::MANIFEST_JSON_COLUMN),
        )
        .optional()?;
    let Some(encoded) = encoded else {
        return Ok(None);
    };
    let manifest: TerminalManifest = serde_json::from_slice(&encoded)?;
    validate_session_manifest(&manifest)?;
    anyhow::ensure!(
        manifest.terminal_id == terminal_id,
        "catalog session identity mismatch"
    );
    Ok(Some(manifest))
}
