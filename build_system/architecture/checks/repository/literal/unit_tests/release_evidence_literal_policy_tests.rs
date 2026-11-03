use yolo_architecture_test_support::Fixture;

const PRIVATE_DIRECTORY_MODE: &str = "700";
const EXECUTABLE_MODE: &str = "0755";

#[test]
fn release_contract_crate_modules_are_a_central_registry() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/release/contracts/src/snapshot.rs",
        "pub const GATE: &str = \"rust_coverage\";\npub const FLAG: &str = \"--strict\";\n",
    );
    fixture.write(
        "build_system/release/verification/src/validation.rs",
        "fn validate() -> bool { true }\n",
    );

    let violations = crate::inspect(fixture.root()).unwrap();
    assert!(violations.is_empty(), "violations: {violations:?}");
}

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_random_shell_cli_env_path_schema_and_timeout_contracts() {
    let fixture = Fixture::new();
    let cli = ["-", "-jobs=7"].concat();
    let environment = ["YOLO_RANDOM_", "999"].concat();
    let path = ["/private/tmp/random-", "123"].concat();
    let schema = ["revision-", "7"].concat();
    fixture.write(
        "plugins/macos/Tests/E2E/random_contract.sh",
        format!(
            "consume \"{cli}\"\nconsume \"{environment}\"\nconsume \"{path}\"\nschema_value=\"{schema}\"\ntimeout 123\n"
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 5, "{result:?}");
    assert!(result.iter().all(|entry| entry.contains("named contract")));
}

#[test]
fn accepts_explicit_shell_contract_declarations() {
    let fixture = Fixture::new();
    fixture.write(
        "plugins/macos/Tests/E2E/release_contract.sh",
        "readonly jobs_argument=\"--jobs=1\"\nreadonly timeout_seconds=7\nrun \"${jobs_argument}\" \"${timeout_seconds}\"\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_inline_shell_permission_modes() {
    let fixture = Fixture::new();
    fixture.write(
        "plugins/macos/Tests/E2E/permission_contract.sh",
        format!(
            "/bin/mkdir -m {PRIVATE_DIRECTORY_MODE} root\n/bin/chmod \"{EXECUTABLE_MODE}\" tool\n"
        ),
    );

    let result = messages(&fixture);
    assert_eq!(result.len(), 2, "{result:?}");
    assert!(result.iter().all(|entry| entry.contains("named contract")));
}

#[test]
fn accepts_named_shell_permission_modes() {
    let fixture = Fixture::new();
    fixture.write(
        "plugins/macos/Tests/E2E/permission_contract.sh",
        format!(
            "readonly YOLO_HOME_MODE={PRIVATE_DIRECTORY_MODE}\nreadonly YOLO_TOOL_MODE={EXECUTABLE_MODE}\n/bin/mkdir -m \"${{YOLO_HOME_MODE}}\" root\n/bin/chmod \"${{YOLO_TOOL_MODE}}\" tool\n"
        ),
    );

    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_random_starlark_contract_literals_but_accepts_named_values() {
    let fixture = Fixture::new();
    let cli = ["-", "-jobs=7"].concat();
    let path = ["/private/tmp/release-", "123"].concat();
    let schema = ["schema-revision-", "999"].concat();
    fixture.write(
        "build_system/platform/macos/release.bzl",
        format!(
            "def release():\n    native.sh_test(\n        args = [\n            \"{cli}\",\n            \"{path}\",\n            \"{schema}\",\n        ],\n        timeout = 123,\n    )\n"
        ),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 4, "{result:?}");
    let named = Fixture::new();
    named.write(
        "build_system/platform/macos/release.bzl",
        "SERIAL_JOBS = \"--jobs=1\"\nRELEASE_PATH = \"/private/tmp/release\"\nSCHEMA = \"schema-v1\"\nTIMEOUT = 7\ndef release():\n    native.sh_test(args = [SERIAL_JOBS, RELEASE_PATH, SCHEMA], timeout = TIMEOUT)\n",
    );
    assert!(messages(&named).is_empty());
}

#[test]
fn rejects_inline_launcher_mode_but_accepts_a_named_contract() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/bazel/rust/launcher.bzl",
        format!("native.genrule(cmd = \"/bin/chmod {EXECUTABLE_MODE} $@\")\n"),
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1, "{result:?}");

    let named = Fixture::new();
    named.write(
        "build_system/bazel/rust/launcher.bzl",
        format!(
            "_EXECUTABLE_MODE = \"{EXECUTABLE_MODE}\"\nnative.genrule(cmd = \"/bin/chmod {{}} $@\".format(_EXECUTABLE_MODE))\n"
        ),
    );
    assert!(messages(&named).is_empty());
}

#[test]
fn rejects_release_rust_contract_strings_outside_constant_sources() {
    let fixture = Fixture::new();
    let cli = ["-", "-jobs=7"].concat();
    let environment = ["YOLO_RANDOM_", "123"].concat();
    let path = ["/private/tmp/release-", "999"].concat();
    let schema = ["schema-revision-", "7"].concat();
    fixture.write(
        "build_system/release/verification/src/validation.rs",
        format!(
            "Command::new(\"{path}\").env(\"{environment}\", \"enabled\").arg(\"{cli}\");\nlet schema = \"{schema}\";\n"
        ),
    );
    let result = messages(&fixture);
    for expected in [&cli, &environment, &path, &schema] {
        assert!(
            result.iter().any(|entry| entry.contains(expected)),
            "{expected}: {result:?}"
        );
    }
}

#[test]
fn permits_non_contract_logs_and_dependency_labels() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/release/verification/src/validation.rs",
        "eprintln!(\"release evidence rejected the candidate\");\n",
    );
    fixture.write(
        "build_system/release/BUILD.bazel",
        "rust_library(srcs = [\"validation.rs\"], deps = [\"//core:yolo_runtime\"])\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn permits_feature_guards_and_explicit_test_source_directories() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/release/src/lib.rs",
        "#[cfg(all(test, feature = \"yolo-unit-tests\"))]\nmod unit_tests;\n",
    );
    fixture.write(
        "build_system/release/unit_tests/mod.rs",
        "#[path = \"../test_support/fixture.rs\"]\nmod fixture;\n",
    );
    fixture.write(
        "build_system/release/verification/test_support/fixture.rs",
        "fn fixed_path() -> &'static str { \"/private/test/fixture\" }\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn permits_rust_module_path_attributes_but_not_runtime_paths() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/release/component/src/lib.rs",
        "#[cfg(test)]\n#[path = \"../unit_tests/mod.rs\"]\nmod unit_tests;\n",
    );
    fixture.write("build_system/release/component/unit_tests/mod.rs", "");
    let allowed = messages(&fixture);
    assert!(allowed.is_empty(), "{allowed:?}");
    fixture.write(
        "build_system/release/component/src/runtime.rs",
        "fn open() { std::fs::read(\"../candidate.json\"); }\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1, "{result:?}");
    assert!(result[0].contains("candidate.json"), "{result:?}");
}
