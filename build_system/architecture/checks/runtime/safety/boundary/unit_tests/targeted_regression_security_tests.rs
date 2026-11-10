use yolo_architecture_test_support::Fixture;

const CONFORMANCE_ROOT_ENV: &str = "YOLO_ARCH_CONFORMANCE_ROOT";
const TEST_SOURCE_DIRECTORY_ENV: &str = "TEST_SRCDIR";
const TEST_WORKSPACE_ENV: &str = "TEST_WORKSPACE";

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_equivalent_octal_hex_and_decimal_permission_masks() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/runtime/engine/host/src/security.rs",
        "assert!(metadata.mode() & 0o077 == 0);\n\
         assert!(metadata.mode() & 0x3f == 0);\n\
         assert!(metadata.mode() & 63 == 0);\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|entry| {
        entry.contains("targeted regression") && entry.contains("named mode mask")
    }));
}

#[test]
fn permits_named_test_and_unrelated_bit_masks() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/runtime/engine/host/src/security.rs",
        "assert!(metadata.mode() & NON_OWNER_PERMISSION_MASK == 0);\n\
         assert!(flags & 0x3f == 0);\n",
    );
    fixture.write(
        "core/crates/persistence/paths/src/tests.rs",
        "assert_eq!(metadata.mode() & 0o777, PRIVATE_DIRECTORY_MODE);\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_skip_and_nth_hashed_host_positions() {
    let fixture = Fixture::new();
    fixture.write(
        crate::targeted_regression_rules::HASHED_HOST_MATCHER_PATH,
        "let fields = pattern.split('|').skip(2);\n\
         let hash = fields.nth(1);\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|entry| entry.contains("format-aware")));
}

#[test]
fn permits_named_or_format_aware_hashed_host_positions() {
    let fixture = Fixture::new();
    fixture.write(
        crate::targeted_regression_rules::HASHED_HOST_MATCHER_PATH,
        "let encoded = pattern.strip_prefix(HASHED_HOST_PREFIX);\n\
         let field = parts.nth(HASH_FIELD_INDEX);\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn configured_repository_contains_the_real_hashed_host_matcher() {
    let root = std::env::var_os(CONFORMANCE_ROOT_ENV)
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let source = std::env::var_os(TEST_SOURCE_DIRECTORY_ENV)?;
            let workspace = std::env::var_os(TEST_WORKSPACE_ENV)?;
            Some(std::path::PathBuf::from(source).join(workspace))
        });
    let Some(root) = root else { return };
    assert!(
        root.join(crate::targeted_regression_rules::HASHED_HOST_MATCHER_PATH)
            .is_file()
    );
}
