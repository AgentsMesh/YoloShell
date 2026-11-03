use super::*;
use crate::model::{DEFAULT_COLS, DEFAULT_ROWS};
use tempfile::tempdir;

const CURRENT_NAME: &str = "current-generation";
const STALE_NAME: &str = "stale-generation";
const CATALOG_FILE_NAME: &str = "catalog.sqlite";
const LOGIN_ARGUMENT: &str = "-l";
const SHELL_COMMAND: &str = "/bin/sh";
const INVALID_JSON: &[u8] = b"invalid-json";

#[test]
fn stale_controller_writes_cannot_mutate_the_current_generation() {
    let directory = tempdir().unwrap();
    let mut catalog = Catalog::open(&directory.path().join(CATALOG_FILE_NAME)).unwrap();
    let stale_generation = catalog.bump_controller_generation().unwrap();
    let mut current = manifest(Uuid::new_v4());
    assert_eq!(
        catalog
            .insert_for_controller(stale_generation, &current)
            .unwrap(),
        ControllerWrite::Applied(())
    );

    let current_generation = catalog.bump_controller_generation().unwrap();
    current.name = CURRENT_NAME.into();
    assert_eq!(
        catalog
            .upsert_for_controller(current_generation, &current)
            .unwrap(),
        ControllerWrite::Applied(())
    );

    let mut stale = current.clone();
    stale.name = STALE_NAME.into();
    assert_eq!(
        catalog
            .upsert_for_controller(stale_generation, &stale)
            .unwrap(),
        ControllerWrite::Fenced
    );
    assert!(matches!(
        catalog
            .archive_for_controller(stale_generation, current.terminal_id, true)
            .unwrap(),
        ControllerWrite::Fenced
    ));
    let absent = manifest(Uuid::new_v4());
    assert_eq!(
        catalog
            .insert_for_controller(stale_generation, &absent)
            .unwrap(),
        ControllerWrite::Fenced
    );

    let stored = catalog.get(current.terminal_id).unwrap().unwrap();
    assert_eq!(stored.name, CURRENT_NAME);
    assert!(!stored.archived);
    assert!(catalog.get(absent.terminal_id).unwrap().is_none());
}

#[test]
fn repair_compare_and_swap_rejects_changed_missing_and_cross_session_rows() {
    let directory = tempdir().unwrap();
    let mut catalog = Catalog::open(&directory.path().join(CATALOG_FILE_NAME)).unwrap();
    let generation = catalog.bump_controller_generation().unwrap();
    let original = manifest(Uuid::new_v4());
    catalog.insert(&original).unwrap();
    let mut current = original.clone();
    current.name = CURRENT_NAME.into();
    catalog.upsert(&current).unwrap();

    let repair = catalog
        .repair_for_controller(
            generation,
            CatalogRowExpectation::Valid(original.clone()),
            &original,
        )
        .unwrap();
    assert_eq!(repair, ControllerWrite::Applied(false));
    catalog.delete(original.terminal_id).unwrap();
    let missing = catalog
        .repair_for_controller(
            generation,
            CatalogRowExpectation::Valid(current.clone()),
            &current,
        )
        .unwrap();
    assert_eq!(missing, ControllerWrite::Applied(false));
    let foreign = manifest(Uuid::new_v4());
    assert!(
        catalog
            .repair_for_controller(generation, CatalogRowExpectation::Valid(foreign), &current,)
            .is_err()
    );
}

#[test]
fn invalid_row_repair_uses_exact_bytes_and_fences_canonical_collisions() {
    let directory = tempdir().unwrap();
    let mut catalog = Catalog::open(&directory.path().join(CATALOG_FILE_NAME)).unwrap();
    let generation = catalog.bump_controller_generation().unwrap();
    let replacement = manifest(Uuid::new_v4());
    let canonical = replacement.terminal_id.to_string();
    insert_raw_row(&catalog, &canonical, INVALID_JSON);
    let changed = vec![INVALID_JSON[0]];
    catalog
        .connection
        .execute(
            constants::UPSERT_SESSION_SQL,
            params![
                canonical,
                changed,
                replacement.updated_at_ms,
                constants::archived_value(replacement.archived)
            ],
        )
        .unwrap();
    let stale = catalog
        .repair_for_controller(
            generation,
            invalid_expectation(&replacement, INVALID_JSON),
            &replacement,
        )
        .unwrap();
    assert_eq!(stale, ControllerWrite::Applied(false));

    catalog.delete(replacement.terminal_id).unwrap();
    let alias = replacement.terminal_id.to_string().to_uppercase();
    insert_raw_row(&catalog, &alias, INVALID_JSON);
    catalog.insert(&replacement).unwrap();
    let collision = catalog
        .repair_for_controller(
            generation,
            CatalogRowExpectation::Invalid {
                row_key: alias,
                manifest_json: INVALID_JSON.to_vec(),
            },
            &replacement,
        )
        .unwrap();
    assert_eq!(collision, ControllerWrite::Applied(false));
}

fn insert_raw_row(catalog: &Catalog, row_key: &str, bytes: &[u8]) {
    catalog
        .connection
        .execute(
            constants::INSERT_SESSION_SQL,
            params![row_key, bytes, now_ms(), constants::archived_value(false)],
        )
        .unwrap();
}

fn invalid_expectation(manifest: &TerminalManifest, bytes: &[u8]) -> CatalogRowExpectation {
    CatalogRowExpectation::Invalid {
        row_key: manifest.terminal_id.to_string(),
        manifest_json: bytes.to_vec(),
    }
}

fn manifest(terminal_id: Uuid) -> TerminalManifest {
    let timestamp = now_ms();
    TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: terminal_id,
        name: STALE_NAME.into(),
        command: SHELL_COMMAND.into(),
        args: vec![LOGIN_ARGUMENT.into()],
        cwd: std::env::current_dir().unwrap(),
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}

include!("generation_fence_alias_tests.rs");
