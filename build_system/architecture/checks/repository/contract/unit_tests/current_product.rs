use super::*;
use yolo_architecture_test_support::Fixture;

const RUST_FILE: &str = "core/crates/example/src/wire.rs";
const TEST_FILE: &str = "core/crates/example/unit_tests/rejections.rs";

#[test]
fn retired_contracts_cannot_return_through_import_aliases_or_definitions() {
    for name in RETIRED_CONTRACTS {
        let fixture = Fixture::new();
        fixture.write(RUST_FILE, format!("use old::{name} as Renamed;"));
        let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
        let mut violations = Vec::new();
        for file in files { inspect(&file, &mut violations); }
        assert!(!violations.is_empty(), "{RUST_FILE}: {name}");
    }
}

#[test]
fn rejection_fixtures_and_explanatory_text_do_not_create_compatibility_routes() {
    let fixture = Fixture::new();
    fixture.write(TEST_FILE, "struct LegacyRecordWire;");
    fixture.write(RUST_FILE, "// Reject WorkerAuthResult\nconst MESSAGE: &str = \"LegacyV3\";");
    let files = yolo_architecture_test_support::load(fixture.root()).unwrap();
    let mut violations = Vec::new();
    for file in files { inspect(&file, &mut violations); }
    assert!(violations.is_empty());
}
