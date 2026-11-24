use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;

fn source(path: &str, text: &str) -> SourceFile {
    SourceFile {
        absolute: path.into(),
        relative: path.to_owned(),
        text: text.to_owned(),
    }
}

fn inspect(files: &[SourceFile]) -> Vec<Violation> {
    let mut violations = Vec::new();
    yolo_architecture_repository::provider_policy::inspect(files, &mut violations);
    violations
}

#[test]
fn retired_provider_policy_paths_are_rejected() {
    for path in [
        "core/crates/agent/system/hooks/installation/catalog/src/lib.rs",
        "core/crates/conversation/providers/src/lib.rs",
        "core/crates/agent/system/providers/src/provider_constants.rs",
        "core/crates/agent/system/providers/src/builtin/qwen.rs",
        "core/crates/plugin/provider/src/constants.rs",
        "core/crates/agent/system/qwen/src/lib.rs",
        "core/crates/conversation/qwen/src/lib.rs",
    ] {
        let violations = inspect(&[source(path, "")]);
        assert_eq!(violations.len(), 1, "{path}");
        assert!(violations[0].message.contains("retired"));
    }
}

#[test]
fn composition_files_may_reference_packages_but_not_literals() {
    let clean = source(
        "core/crates/agent/system/providers/src/builtin.rs",
        "yolo_qwen_provider::runtime()",
    );
    assert!(inspect(&[clean]).is_empty());
    let literal = source(
        "core/crates/agent/system/providers/src/builtin.rs",
        "let token = \"qwen_code\";",
    );
    assert_eq!(inspect(&[literal]).len(), 1);
}

#[test]
fn shared_production_files_must_not_carry_provider_literals() {
    let literal = source(
        "core/crates/agent/system/providers/src/lib.rs",
        "pub const TOKEN: &str = \"gemini_cli\";",
    );
    assert_eq!(inspect(&[literal]).len(), 1);
    let identifier = source(
        "core/crates/agent/system/providers/src/lib.rs",
        "yolo_gemini_provider::runtime()",
    );
    assert!(inspect(&[identifier]).is_empty());
}

#[test]
fn provider_packages_wire_constants_and_tests_are_exempt() {
    for path in [
        "core/crates/agent/providers/gemini/provider/src/constants.rs",
        "core/crates/agent/providers/qwen/parser/src/constants.rs",
        "core/crates/conversation/protocol/src/constants.rs",
        "core/crates/agent/system/providers/unit_tests/composition_tests.rs",
    ] {
        let file = source(path, "pub const TOKEN: &str = \"gemini_cli\";");
        assert!(inspect(&[file]).is_empty(), "{path}");
    }
}
