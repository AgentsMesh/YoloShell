use super::fixture::Fixture;
use super::process_signal_test_support::messages;

const PARENT_PATH: &str = "core/crates/runtime/engine/process/src/future.rs";
const TEST_PATH: &str = "core/crates/runtime/engine/process/src/future_tests.rs";
const NESTED_PATH: &str = "core/crates/runtime/engine/process/src/tests/hidden.rs";
const HOLDER_FILE: &str = "pty_session_linux_integration_holder.rs";
const MODULE_POLICY: &str = "test-named Rust modules require";

#[test]
fn rejects_test_named_or_test_directory_modules_in_production() {
    for declaration in [
        "mod future_tests;\n",
        "#[path = \"future_tests.rs\"] mod future;\n",
        "#[path = \"tests/hidden.rs\"] mod hidden;\n",
        "include!(\"tests/hidden.rs\");\n",
        "include!(concat!(\"tests/\", \"hidden.rs\"));\n",
        "#[cfg(any(test, target_os = \"linux\"))] mod future_tests;\n",
        "#[path = \"pty_session_linux_integration_holder.rs\"] mod holder;\n",
    ] {
        let fixture = Fixture::new();
        fixture.write(PARENT_PATH, declaration);
        fixture.write(TEST_PATH, "fn helper() { child.kill(); }\n");
        fixture.write(NESTED_PATH, "fn helper() { child.kill(); }\n");
        assert!(
            contains(&messages(&fixture), MODULE_POLICY),
            "{declaration}"
        );
    }
}

#[test]
fn accepts_test_sources_behind_a_production_excluding_guard() {
    let fixture = Fixture::new();
    fixture.write(
        PARENT_PATH,
        "#[cfg(all(test, target_os = \"linux\"))]\n#[path = \"future_tests.rs\"]\nmod future;\n",
    );
    fixture.write(TEST_PATH, "fn helper() { child.kill(); }\n");
    let result = messages(&fixture);
    assert!(result.is_empty(), "{result:#?}");
}

#[test]
fn accepts_test_sources_behind_a_feature_conjoined_test_guard() {
    let fixture = Fixture::new();
    fixture.write(
        PARENT_PATH,
        "#[cfg(all(test, feature = \"yolo-unit-tests\"))]\n#[path = \"future_tests.rs\"]\nmod future;\n",
    );
    fixture.write(TEST_PATH, "fn helper() { child.kill(); }\n");
    let result = messages(&fixture);
    assert!(result.is_empty(), "{result:#?}");
}

#[test]
fn holder_name_is_not_a_production_wiring_escape() {
    let fixture = Fixture::new();
    fixture.write(
        PARENT_PATH,
        &format!("#[path = \"{HOLDER_FILE}\"] mod holder;\n"),
    );
    assert!(contains(&messages(&fixture), MODULE_POLICY));
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
