const NONCANONICAL_SESSION_ID: &str = "abcdef01-2345-4678-9abc-def012345678";
const EXPECTED_DIAGNOSTIC_COUNT: usize = 1;
const EMPTY_ROW_COUNT: i64 = 0;
const ROW_COUNT_SQL: &str = "SELECT COUNT(*) FROM sessions WHERE session_id = ?1";

#[test]
fn noncanonical_valid_row_is_repaired_to_the_canonical_key() {
    let directory = tempdir().unwrap();
    let mut catalog = Catalog::open(&directory.path().join(CATALOG_FILE_NAME)).unwrap();
    let generation = catalog.bump_controller_generation().unwrap();
    let durable = alias_manifest();
    let encoded = serde_json::to_vec(&durable).unwrap();
    let alias = noncanonical_key(&durable);
    insert_raw_row(&catalog, &alias, &encoded);

    let expectation = alias_expectation(&catalog, &durable, &alias);
    let repaired = catalog
        .repair_for_controller(generation, expectation, &durable)
        .unwrap();

    assert_eq!(repaired, ControllerWrite::Applied(true));
    assert_eq!(raw_row_count(&catalog, &alias), EMPTY_ROW_COUNT);
    let stored = catalog.get(durable.terminal_id).unwrap().unwrap();
    assert_eq!(serde_json::to_vec(&stored).unwrap(), encoded);
}

#[test]
fn noncanonical_repair_cas_preserves_a_concurrent_alias_update() {
    let directory = tempdir().unwrap();
    let mut catalog = Catalog::open(&directory.path().join(CATALOG_FILE_NAME)).unwrap();
    let generation = catalog.bump_controller_generation().unwrap();
    let durable = alias_manifest();
    let alias = noncanonical_key(&durable);
    insert_raw_row(&catalog, &alias, &serde_json::to_vec(&durable).unwrap());
    let expectation = alias_expectation(&catalog, &durable, &alias);

    let mut concurrent = durable.clone();
    concurrent.name = CURRENT_NAME.into();
    let concurrent_bytes = serde_json::to_vec(&concurrent).unwrap();
    catalog
        .connection
        .execute(
            constants::UPSERT_SESSION_SQL,
            params![
                alias,
                concurrent_bytes,
                concurrent.updated_at_ms,
                constants::archived_value(concurrent.archived)
            ],
        )
        .unwrap();
    let repaired = catalog
        .repair_for_controller(generation, expectation, &durable)
        .unwrap();

    assert_eq!(repaired, ControllerWrite::Applied(false));
    assert!(catalog.get(durable.terminal_id).unwrap().is_none());
    assert_eq!(raw_manifest(&catalog, &alias), concurrent_bytes);
}

fn alias_expectation(
    catalog: &Catalog,
    manifest: &TerminalManifest,
    alias: &str,
) -> CatalogRowExpectation {
    let mut listing = catalog.list_tolerant(true).unwrap();
    assert!(listing.manifests.is_empty());
    assert_eq!(listing.diagnostics.len(), EXPECTED_DIAGNOSTIC_COUNT);
    let diagnostic = listing.diagnostics.pop().unwrap();
    assert_eq!(diagnostic.terminal_id, Some(manifest.terminal_id));
    assert_eq!(diagnostic.row_key, alias);
    diagnostic.expectation.unwrap()
}

fn alias_manifest() -> TerminalManifest {
    manifest(Uuid::parse_str(NONCANONICAL_SESSION_ID).unwrap())
}

fn noncanonical_key(manifest: &TerminalManifest) -> String {
    let key = manifest.terminal_id.to_string().to_uppercase();
    assert_ne!(key, manifest.terminal_id.to_string());
    key
}

fn raw_row_count(catalog: &Catalog, row_key: &str) -> i64 {
    catalog
        .connection
        .query_row(ROW_COUNT_SQL, [row_key], |row| row.get(0))
        .unwrap()
}

fn raw_manifest(catalog: &Catalog, row_key: &str) -> Vec<u8> {
    catalog
        .connection
        .query_row(constants::READ_SESSION_SQL, [row_key], |row| row.get(0))
        .unwrap()
}
