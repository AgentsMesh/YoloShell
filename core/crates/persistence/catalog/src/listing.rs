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

#[derive(Debug)]
pub struct CatalogListing {
    pub manifests: Vec<TerminalManifest>,
    pub diagnostics: Vec<CatalogDiagnostic>,
}

impl Catalog {
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
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(constants::SESSION_ID_COLUMN)?,
                row.get::<_, Vec<u8>>(constants::MANIFEST_JSON_COLUMN)?,
                row.get::<_, i64>(constants::UPDATED_AT_COLUMN)?,
                row.get::<_, i64>(constants::ARCHIVED_COLUMN)?,
            ))
        })?;
        let mut listing = CatalogListing {
            manifests: Vec::new(),
            diagnostics: Vec::new(),
        };
        for row in rows {
            match row {
                Ok((key, bytes, updated_at_ms, archived)) => push_catalog_row(
                    &mut listing,
                    key,
                    &bytes,
                    updated_at_ms,
                    archived,
                    include_archived,
                ),
                Err(error) => listing.diagnostics.push(CatalogDiagnostic {
                    terminal_id: None,
                    row_key: constants::UNREADABLE_ROW_KEY.into(),
                    detail: error.to_string(),
                    expectation: None,
                }),
            }
        }
        Ok(listing)
    }
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
    let parsed = serde_json::from_slice::<TerminalManifest>(bytes);
    let expectation = CatalogRowExpectation::Invalid {
        row_key: row_key.clone(),
        manifest_json: bytes.to_vec(),
    };
    match parsed {
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
        }
        Ok(manifest) => listing.diagnostics.push(CatalogDiagnostic {
            terminal_id,
            row_key,
            detail: constants::inconsistent_manifest_row_detail(&manifest, updated_at_ms, archived),
            expectation: Some(expectation),
        }),
        Err(error) => listing.diagnostics.push(CatalogDiagnostic {
            terminal_id,
            row_key,
            detail: error.to_string(),
            expectation: Some(expectation),
        }),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/listing_adversarial_tests.rs"]
mod listing_adversarial_tests;

fn validate_session_manifest(manifest: &TerminalManifest) -> Result<()> {
    if manifest.schema_version != SESSION_MANIFEST_SCHEMA_VERSION {
        bail!(constants::unsupported_manifest_schema_detail(
            manifest.schema_version
        ));
    }
    Ok(())
}
