use yolo_architecture_test_support::Fixture;

fn joined(parts: &[&str]) -> String {
    parts.concat()
}

#[test]
fn rejects_central_fault_reap_values_repeated_as_swift_enum_literals() {
    let fixture = Fixture::new();
    let cleanup = joined(&["cleanup", "_advanced"]);
    let observer = joined(&["observer", "_only"]);
    fixture.write(
        "plugins/macos/Sources/Feature/WireConstants.swift",
        format!(
            "enum WireValue {{ static let cleanup = \"{cleanup}\"; static let observer = \"{observer}\" }}\n"
        ),
    );
    fixture.write(
        "plugins/macos/Sources/Feature/FaultReapWire.swift",
        format!(
            "enum Kind: String, Decodable {{ case cleanup = \"{cleanup}\"; case observer = \"{observer}\" }}\n"
        ),
    );
    let result = crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect::<Vec<_>>();
    assert_eq!(result.len(), 2);
    assert!(
        result
            .iter()
            .all(|entry| entry.contains("centralized constant"))
    );
}
