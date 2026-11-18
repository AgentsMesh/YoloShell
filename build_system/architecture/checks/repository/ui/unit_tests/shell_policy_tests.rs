use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_native_application_launch_commands_from_shell_tests() {
    let fixture = Fixture::new();
    fixture.write(
        "plugins/macos/Tests/E2E/focus_test.sh",
        "/usr/bin/open -a Yolo\nosascript -e 'tell application \"Yolo\" to activate'\n/bin/launchctl kickstart gui/501/yolo\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|entry| entry.contains("must not launch")));
}

#[test]
fn rejects_wrapped_or_chained_gui_launch_commands() {
    let fixture = Fixture::new();
    fixture.write(
        "integration_tests/gui.sh",
        "true && exec /usr/bin/open Yolo.app\nif env MODE=test osascript -e quit; then true; fi\ncommand launchctl print gui/501\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
}

#[test]
fn permits_native_binary_path_and_non_command_mentions_without_execution() {
    let fixture = Fixture::new();
    fixture.write(
        "plugins/macos/Tests/E2E/inactive_launch_test.sh",
        "readonly app_binary=\"${test_root}/Yolo.app/Contents/MacOS/Yolo\"\necho \"open is forbidden here\"\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn bundle_audit_must_invoke_the_native_bundle_policy() {
    let fixture = Fixture::new();
    fixture.write("plugins/macos/Tests/E2E/bundle_audit_test.sh", "true\n");
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("native payload and dependency policy"));

    let fixture = Fixture::new();
    fixture.write(
        "plugins/macos/Tests/E2E/bundle_audit_test.sh",
        "# enforce_native_bundle_policy is required\nenforce_native_bundle_policy \"${app}\" file otool\n",
    );
    assert!(messages(&fixture).is_empty());
}
