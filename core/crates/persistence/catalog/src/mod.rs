include!("module_wiring.rs");

use crate::constants::SESSION_MANIFEST_SCHEMA_VERSION;

use crate::model::{RecoveryState, TerminalManifest, now_ms};

use anyhow::{Context, Result, bail};

use rusqlite::{Connection, OptionalExtension, params};

use std::path::Path;

use uuid::Uuid;

pub struct Catalog {

    connection: Connection,

    _retained: Option<retained_namespace::RetainedCatalogNamespace>,

}

impl Catalog {

    pub fn insert(&mut self, manifest: &TerminalManifest) -> Result<()> {

        validate_session_manifest(manifest)?;

        let encoded = serde_json::to_vec(manifest)?;

        self.connection.execute(

            constants::INSERT_SESSION_SQL,

            params![

                manifest.terminal_id.to_string(),

                encoded,

                manifest.updated_at_ms,

                constants::archived_value(manifest.archived)

            ],

        )?;

        Ok(())

    }

    pub fn upsert(&mut self, manifest: &TerminalManifest) -> Result<()> {

        validate_session_manifest(manifest)?;

        let encoded = serde_json::to_vec(manifest)?;

        self.connection.execute(

            constants::UPSERT_SESSION_SQL,

            params![

                manifest.terminal_id.to_string(),

                encoded,

                manifest.updated_at_ms,

                constants::archived_value(manifest.archived)

            ],

        )?;

        Ok(())

    }

    pub fn get(&self, id: Uuid) -> Result<Option<TerminalManifest>> {

        let bytes: Option<Vec<u8>> = self

            .connection

            .query_row(constants::READ_SESSION_SQL, [id.to_string()], |row| {

                row.get(constants::MANIFEST_JSON_COLUMN)

            })

            .optional()?;

        let Some(bytes) = bytes else {

            return Ok(None);

        };

        let manifest: TerminalManifest = serde_json::from_slice(&bytes)?;

        validate_session_manifest(&manifest)?;

        anyhow::ensure!(

            manifest.terminal_id == id,

            "catalog session identity mismatch"

        );

        Ok(Some(manifest))

    }

    pub fn archive(&mut self, id: Uuid, archived: bool) -> Result<Option<TerminalManifest>> {

        let Some(mut manifest) = self.get(id)? else {

            return Ok(None);

        };

        manifest.archived = archived;

        manifest.updated_at_ms = now_ms();

        self.upsert(&manifest)?;

        Ok(Some(manifest))

    }

    pub fn update_recovery(

        &mut self,

        id: Uuid,

        state: RecoveryState,

    ) -> Result<Option<TerminalManifest>> {

        let Some(mut manifest) = self.get(id)? else {

            return Ok(None);

        };

        manifest.recovery_state = state;

        manifest.updated_at_ms = now_ms();

        self.upsert(&manifest)?;

        Ok(Some(manifest))

    }

    pub fn delete(&mut self, id: Uuid) -> Result<bool> {

        Ok(self

            .connection

            .execute(constants::DELETE_SESSION_SQL, [id.to_string()])?

            > 0)

    }

}

#[cfg(test)]
pub(crate) use constants::{

    ARCHIVED_COLUMN as CATALOG_ARCHIVED_COLUMN, CONTROLLER_GENERATION_EXHAUSTED,

    MANIFEST_JSON_COLUMN as CATALOG_MANIFEST_JSON_COLUMN,

    SESSION_ID_COLUMN as CATALOG_SESSION_ID_COLUMN, UPDATED_AT_COLUMN as CATALOG_UPDATED_AT_COLUMN,

};

pub use generation_fence::ControllerWrite;

pub use catalog_alias_scanner::CatalogAliasScanner;

pub use open::CatalogRepairEntry;

include!("listing.rs");

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/module_wiring.rs");
