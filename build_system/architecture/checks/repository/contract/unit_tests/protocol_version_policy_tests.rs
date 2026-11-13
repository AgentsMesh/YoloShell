use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn accepts_central_versions_named_offsets_and_references() {
    let fixture = Fixture::new();
    fixture.write(
        "core/src/model.rs",
        "pub const PROTOCOL_VERSION: u16 = 1;\n",
    );
    fixture.write(
        "build_system/release/git_constants.rs",
        "pub const PRIVATE_PROTOCOL_VERSION: u16 = 1;\n",
    );
    fixture.write(
        "core/src/service.rs",
        "const INCOMPATIBLE_VERSION_OFFSET: u16 = 1;\nlet version = PROTOCOL_VERSION + INCOMPATIBLE_VERSION_OFFSET;\n",
    );
    fixture.write(
        "plugins/macos/Tests/Feature/WireConstants.swift",
        "enum WireValue { static let protocolVersion = 1 }\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_direct_protocol_versions_and_adjustments() {
    let fixture = Fixture::new();
    fixture.write(
        "core/src/service.rs",
        "let status = Status { protocol_version: 1 };\nstatus.protocol_version += 1;\nassert_eq!(decoded.version, 1);\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert!(
        result
            .iter()
            .all(|message| message.contains("centralized constants"))
    );
}

#[test]
fn rejects_protocol_declaration_outside_constant_sources() {
    let fixture = Fixture::new();
    fixture.write(
        "plugins/macos/Tests/Feature/WireTests.swift",
        "private enum Value { static let protocolVersion = 1 }\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
}
