use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_inline_numeric_durations_in_product_and_test_sources() {
    let fixture = Fixture::new();
    let rust_seconds = ["Duration::from_", "secs(5)"].concat();
    let rust_millis = ["Duration::from_", "millis(25)"].concat();
    let rust_micros = ["Duration::from_", "micros(50)"].concat();
    let rust_nanos = ["Duration::from_", "nanos(75)"].concat();
    fixture.write(
        "core/src/service.rs",
        format!("consume({rust_seconds});\nconsume({rust_millis});\n"),
    );
    fixture.write(
        "core/src/service_tests.rs",
        format!("consume({rust_micros});\nconsume({rust_nanos});\n"),
    );
    let swift_seconds = [".", "seconds(5)"].concat();
    let swift_millis = [".", "milliseconds(25)"].concat();
    let swift_micros = [".", "microseconds(50)"].concat();
    let swift_nanos = [".", "nanoseconds(75)"].concat();
    fixture.write(
        "plugins/macos/Tests/YoloWorkspaceTests/ServiceTests.swift",
        format!(
            "consume({swift_seconds})\nconsume({swift_millis})\nconsume({swift_micros})\nconsume({swift_nanos})\n"
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 8);
    assert!(result.iter().all(|entry| entry.contains("named constant")));
}

#[test]
fn rejects_random_operational_numbers_without_a_value_blacklist() {
    let fixture = Fixture::new();
    let rust = [
        ["Duration::from_", "millis(999);"].concat(),
        ["std::thread::", "sleep(7);"].concat(),
        ["Vec::with_", "capacity(123);"].concat(),
        "let retry_limit = 1;".to_owned(),
        "let buffer_capacity = 0;".to_owned(),
    ]
    .join("\n");
    fixture.write("core/src/service.rs", rust);
    let swift = [
        ["consume(.", "milliseconds(123))"].concat(),
        ["try await Task.", "sleep(nanoseconds: 999)"].concat(),
        "let retryLimit = 7".to_owned(),
        "let bufferCapacity = 1".to_owned(),
    ]
    .join("\n");
    fixture.write("plugins/macos/Sources/YoloWorkspace/Service.swift", swift);
    let result = messages(&fixture);
    assert_eq!(result.len(), 9, "{result:?}");
    assert!(result.iter().all(|entry| entry.contains("named constant")));
}

#[test]
fn permits_tuple_indexes_and_numbers_inside_closure_bodies() {
    let fixture = Fixture::new();
    fixture.write(
        "core/src/service.rs",
        "let lower = bytes.size_hint().0;\n\
         let state = records[index].0;\n\
         timeout(NAMED_TIMEOUT, async {\n\
             if last_sequence() > 0 { observe(); }\n\
         }).await;\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_declaration_text_bypasses() {
    let fixture = Fixture::new();
    let rust_duration = ["Duration::from_", "secs(5)"].concat();
    fixture.write(
        "core/src/service_tests.rs",
        format!(
            "let marker = \"const \"; consume({rust_duration});\n// const FAKE: Duration =\nconsume({rust_duration});\nconsume(\"static \", {rust_duration});\nconst fn fake() -> Duration {{ {rust_duration} }}\n"
        ),
    );
    let swift_seconds = [".", "seconds(5)"].concat();
    let swift_millis = [".", "milliseconds(25)"].concat();
    fixture.write(
        "plugins/macos/Tests/YoloWorkspaceTests/ServiceTests.swift",
        format!(
            "consume(\"static let \", {swift_seconds})\nconsume({swift_millis}) // public static let fake\n"
        ),
    );
    assert_eq!(messages(&fixture).len(), 6);
}

#[test]
fn permits_named_declarations_and_centralized_swift_timings() {
    let fixture = Fixture::new();
    let rust_millis = ["Duration::from_", "millis(25)"].concat();
    let rust_seconds = ["Duration::from_", "secs(5)"].concat();
    fixture.write(
        "core/src/service_tests.rs",
        format!(
            "pub const RETRY_DELAY: Duration =\n    {rust_millis};\npub(crate) static TIMEOUT: Duration =\n    {rust_seconds};\n"
        ),
    );
    fixture.write(
        "plugins/macos/Tests/YoloWorkspaceTests/ServiceTests.swift",
        "private static let retryDelay: Duration = .milliseconds(25)\npublic static let timeout: Duration = .seconds(5)\n",
    );
    let centralized_millis = [".", "milliseconds(25)"].concat();
    fixture.write(
        "plugins/macos/Sources/YoloWorkspace/ServicePolicy.swift",
        format!("private static let retryDelay: Duration = {centralized_millis}\n"),
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn swift_closure_parameters_and_standalone_numbers_are_not_operational_literals() {
    for (relative, text) in [
        ("sample.swift", "values.map { delay($0) }; tuple.0"),
        ("sample.rs", "1; value.0"),
        ("sample.txt", "sleep(99)"),
    ] {
        let file = yolo_architecture_foundation::SourceFile {
            absolute: relative.into(),
            relative: relative.into(),
            text: text.into(),
        };
        let mut violations = Vec::new();
        crate::timing_rules::inspect(&file, &mut violations);
        assert!(violations.is_empty(), "{text}: {violations:?}");
    }
}
