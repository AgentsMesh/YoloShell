const LIST_ALL_QUERY: &str = "SELECT session_id, manifest_json, updated_at_ms, archived \
     FROM sessions ORDER BY updated_at_ms DESC, session_id";

#[derive(Debug)]
pub struct CatalogDiagnostic {
    pub terminal_id: Option<Uuid>,
    pub row_key: String,
    pub detail: String,
    pub expectation: Option<CatalogRowExpectation>,
}

#[derive(Debug)]
pub enum CatalogRowExpectation {
    Valid(TerminalManifest),
    Invalid {
        row_key: String,
        manifest_json: Vec<u8>,
    },
}

#[derive(Debug, Default)]
pub struct CatalogListing {
    pub manifests: Vec<TerminalManifest>,
    pub diagnostics: Vec<CatalogDiagnostic>,
}

impl Catalog {
    pub fn get(&self, id: Uuid) -> Result<Option<TerminalManifest>> {
        let Some(bytes) = self
            .connection
            .query_row(constants::READ_SESSION_SQL, [id.to_string()], |row| {
                row.get::<_, Vec<u8>>(constants::MANIFEST_JSON_COLUMN)
            })
            .optional()? else {
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

    pub fn list(&self, include_archived: bool) -> Result<Vec<TerminalManifest>> {
        let listing = self.list_tolerant(include_archived)?;
        if let Some(diagnostic) = listing.diagnostics.first() {
            bail!(constants::invalid_catalog_session_detail(
                &diagnostic.row_key,
                &diagnostic.detail
            ));
        }
        Ok(listing.manifests)
    }

    pub fn list_tolerant(&self, include_archived: bool) -> Result<CatalogListing> {
        let mut statement = self.connection.prepare(LIST_ALL_QUERY)?;
        let mut listing = CatalogListing::default();
        read_catalog_rows(&mut statement, [], include_archived, None, &mut listing)?;
        Ok(listing)
    }

    /// The watchdog already knows which terminal identities need another visit.
    pub fn list_selected(&self, ids: &[Uuid]) -> Result<CatalogListing> {
        let mut statement = self.connection.prepare_cached(
            "SELECT session_id, manifest_json, updated_at_ms, archived FROM sessions WHERE session_id = ?1",
        )?;
        let mut listing = CatalogListing::default();
        for &id in ids {
            read_catalog_rows(
                &mut statement,
                [id.to_string()],
                true,
                Some(id),
                &mut listing,
            )?;
        }
        Ok(listing)
    }
}

fn read_catalog_rows(
    statement: &mut rusqlite::Statement<'_>,
    params: impl rusqlite::Params,
    archived: bool,
    selected: Option<Uuid>,
    listing: &mut CatalogListing,
) -> Result<()> {
    let rows = statement.query_map(params, |row| {
        Ok((
            row.get::<_, String>(constants::SESSION_ID_COLUMN)?,
            row.get::<_, Vec<u8>>(constants::MANIFEST_JSON_COLUMN)?,
            row.get::<_, i64>(constants::UPDATED_AT_COLUMN)?,
            row.get::<_, i64>(constants::ARCHIVED_COLUMN)?,
        ))
    })?;
    for row in rows {
        match row {
            Ok((key, bytes, updated, flag)) => {
                push_catalog_row(listing, key, &bytes, updated, flag, archived)
            }
            Err(error) => listing.diagnostics.push(CatalogDiagnostic {
                terminal_id: selected,
                row_key: constants::UNREADABLE_ROW_KEY.into(),
                detail: error.to_string(),
                expectation: None,
            }),
        }
    }
    Ok(())
}

fn push_catalog_row(
    listing: &mut CatalogListing,
    row_key: String,
    bytes: &[u8],
    updated_at_ms: i64,
    archived: i64,
    include_archived: bool,
) {
    let terminal_id = Uuid::parse_str(&row_key).ok();
    let detail = match serde_json::from_slice::<TerminalManifest>(bytes) {
        Ok(manifest)
            if Some(manifest.terminal_id) == terminal_id
                && row_key == manifest.terminal_id.to_string()
                && manifest.schema_version == SESSION_MANIFEST_SCHEMA_VERSION
                && matches!(serde_json::to_vec(&manifest), Ok(canonical) if canonical == bytes)
                && updated_at_ms == manifest.updated_at_ms
                && archived == constants::archived_value(manifest.archived) =>
        {
            if include_archived || !manifest.archived {
                listing.manifests.push(manifest);
            }
            return;
        }
        Ok(manifest) => {
            constants::inconsistent_manifest_row_detail(&manifest, updated_at_ms, archived)
        }
        Err(error) => error.to_string(),
    };
    let expectation = CatalogRowExpectation::Invalid {
        row_key: row_key.clone(),
        manifest_json: bytes.to_vec(),
    };
    listing.diagnostics.push(CatalogDiagnostic {
        terminal_id,
        row_key,
        detail,
        expectation: Some(expectation),
    });
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/listing_adversarial_tests.rs"]
mod listing_adversarial_tests;

fn validate_session_manifest(manifest: &TerminalManifest) -> Result<()> {
    anyhow::ensure!(manifest.schema_version == SESSION_MANIFEST_SCHEMA_VERSION,
        constants::unsupported_manifest_schema_detail(manifest.schema_version));
    Ok(())
}
