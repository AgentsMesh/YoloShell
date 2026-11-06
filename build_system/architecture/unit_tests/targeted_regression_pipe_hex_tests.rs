use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_multiply_and_shift_hex_capacities() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/persistence/paths/src/hash.rs",
        "String::with_capacity(digest.len() * 2);\n\
         String::with_capacity(hash.len() << 1);\n",
    );
    fixture.write(
        "core/src/coordination/mach_lock.rs",
        "String::with_capacity(Sha256::output_size() * 2);\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 6);
    assert_eq!(
        result
            .iter()
            .filter(|entry| entry.contains("encoded-length constant"))
            .count(),
        3
    );
    assert_eq!(
        result
            .iter()
            .filter(|entry| entry.contains("named constant"))
            .count(),
        3
    );
}

#[test]
fn permits_named_and_unrelated_capacities() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/persistence/paths/src/hash.rs",
        "const GROWTH_SHIFT: usize = 1;\n\
         String::with_capacity(SOCKET_FULL_HASH_HEX_LENGTH);\n\
         Vec::with_capacity(bytes.len() << GROWTH_SHIFT);\n",
    );
    assert!(messages(&fixture).is_empty());
}
