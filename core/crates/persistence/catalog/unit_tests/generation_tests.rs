use super::*;
use rusqlite::ToSql;

const GENERATION_UPDATE_SQL: &str =
    "UPDATE metadata SET value = ?1 WHERE key = 'controller_generation'";
const NONCANONICAL_GENERATIONS: &[&str] = &[
    "",
    "-1",
    "+1",
    " 1",
    "01",
    "1.0",
    "1e0",
    "18446744073709551616",
];

#[test]
fn noncanonical_controller_generations_are_rebuildable() {
    for generation in NONCANONICAL_GENERATIONS {
        assert_generation_is_rebuildable(generation);
    }
    assert_generation_is_rebuildable(&b"1".as_slice());
}

#[test]
fn maximum_controller_generation_is_preserved_and_exhausted() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite");
    drop(Catalog::open(&path).unwrap());
    overwrite_generation(&path, &u64::MAX.to_string());
    let mut catalog = Catalog::open(&path).unwrap();

    let error = catalog.bump_controller_generation().unwrap_err();

    assert!(
        error
            .to_string()
            .contains("controller generation exhausted")
    );
    assert_eq!(catalog.controller_generation().unwrap(), u64::MAX);
}

fn assert_generation_is_rebuildable(generation: &dyn ToSql) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.sqlite");
    drop(Catalog::open(&path).unwrap());
    overwrite_generation(&path, generation);

    let error = Catalog::open(&path).err().unwrap();

    assert!(rebuild_reason::is_rebuildable(&error));
}

fn overwrite_generation(path: &Path, generation: &dyn ToSql) {
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(GENERATION_UPDATE_SQL, [generation])
        .unwrap();
}
