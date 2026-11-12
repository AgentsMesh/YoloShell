use yolo_architecture_test_support::Fixture;

#[test]
fn counts_transitive_includes_in_the_module_scope() {
    let fixture = Fixture::new();
    let mut composition = String::new();
    for group in 0..2 {
        let mut aggregate = String::new();
        for index in 0..4 {
            fixture.write(
                &format!("core/crates/example/src/generated/{group}_{index}.rs"),
                &"fn placeholder() {}\n".repeat(150),
            );
            aggregate.push_str(&format!("include!(\"{group}_{index}.rs\");\n"));
        }
        fixture.write(
            &format!("core/crates/example/src/generated/aggregate_{group}.rs"),
            &aggregate,
        );
        composition.push_str(&format!("include!(\"generated/aggregate_{group}.rs\");\n"));
    }
    fixture.write("core/crates/example/src/mod.rs", &composition);

    let result = super::messages(&fixture);
    assert!(
        result.iter().any(|entry| {
            entry.contains("module scope exceeds")
                && entry.contains("core/crates/example/src/mod.rs")
        }),
        "{result:?}"
    );
}
