use crate::policy_exemptions::{CARGO_RULE_OWNER, CARGO_TEST_OWNER};
use yolo_architecture_test_support::Fixture;

fn messages(fixture: &Fixture) -> Vec<String> {
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn rejects_direct_wrapped_and_environment_cargo_execution() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/build.sh",
        "echo cargo\nenv RUSTFLAGS=warn cargo test\n/usr/bin/cargo build\n\
         command cargo check\nbash -c 'cargo metadata'\nzsh -c \"cargo clippy\"\n\
         \"$CARGO\" fmt\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 6);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn rejects_shell_variables_defaults_and_string_concatenation() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/variables.bash",
        "TOOL=cargo\n\"$TOOL\" test\n${UNSET:-cargo} check\ncar\"go\" fmt\n",
    );
    fixture.write(
        "build_system/defaults.zsh",
        "TOOL=${MISSING:-cargo}\n${TOOL} metadata\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 4);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn rejects_shell_control_words_and_wrapper_options_without_flagging_output() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/control.sh",
        "if cargo test; then :; fi\nwhile cargo check; do break; done\nuntil cargo metadata; do break; done\n\
         ! cargo fmt\ntime cargo clippy\nenv -i cargo test\ncommand -p cargo check\n\
         exec -a build cargo metadata\nenv -S 'cargo test'\necho cargo\nprintf '%s' cargo\n\
         command -v cargo\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 9);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn rejects_shell_command_strings_in_combined_short_options_only() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/wrappers.sh",
        "bash -lc 'cargo test'\nzsh -cl 'cargo check'\nsh -l cargo\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn rejects_common_cargo_launchers_and_their_path_forms() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/launchers.sh",
        "sudo -u root cargo test\n/usr/bin/nice -n 5 /opt/bin/cargo check\n\
         nohup cargo metadata\n/usr/bin/xcrun --sdk macosx cargo fmt\n\
         xcrun --find cargo\necho sudo cargo\nprintf '%s' 'xcrun cargo'\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 4);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn rejects_eval_substitutions_and_field_split_variable_commands() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/dynamic.sh",
        "eval 'cargo test'\necho `cargo metadata`\nprintf '%s' \"$(cargo check)\"\n\
         CMD='cargo fmt'; $CMD\necho '$(cargo test)'\nprintf '%s' cargo\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 4);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn scans_extensionless_shell_shebangs() {
    let fixture = Fixture::new();
    fixture.write("build_system/build", "#!/usr/bin/env bash\ncargo test\n");
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("execute Cargo"));
}

#[test]
fn starlark_checks_only_command_attributes_and_resolves_concatenation() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/BUILD.bazel",
        "TOOL = \"cargo\"\n\
         genrule(name = \"bad\", cmd = TOOL + \" test\", description = \"cargo check\")\n\
         cmd = \"car\" + \"go fmt\"\n\
         command = \"${UNSET:-cargo} metadata\"\n\
         executor = \"/usr/bin/cargo\"\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn starlark_scans_each_list_select_and_dictionary_command_branch() {
    let fixture = Fixture::new();
    fixture.write(
        "build_system/BUILD.bazel",
        "commands = [\"echo ok\", \"cargo test\"]\n\
         cmd = select({\":mac\": [\"echo safe\", \"cargo check\"], \
         \"//conditions:default\": \"echo default\"})\n\
         command = {\"safe\": \"echo ok\", \"build\": \"cargo metadata\"}\n\
         commands = [\"echo cargo\", \"printf '%s' cargo\"]\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|entry| entry.contains("execute Cargo")));
}

#[test]
fn permits_declarations_descriptions_and_non_executing_shell_mentions() {
    let fixture = Fixture::new();
    fixture.write(
        "MODULE.bazel",
        "crate.from_cargo(name = \"crates\", cargo_lockfile = \"//:Cargo.lock\")\n\
         description = \"cargo test\"\n",
    );
    fixture.write(
        "build_system/build.sh",
        "TOOL=cargo\necho \"$TOOL is not executed\"\nprintf '%s' cargo\n",
    );
    assert!(messages(&fixture).is_empty());
}

#[test]
fn rejects_cargo_test_binary_environment_discovery() {
    let fixture = Fixture::new();
    fixture.write(
        "integration_tests/runtime/resilience.rs",
        "const HOST: Option<&str> = option_env!(\"CARGO_BIN_EXE_yolo-host\");\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("Bazel inputs"));
}

#[test]
fn checker_fixture_exemption_is_exact() {
    let fixture = Fixture::new();
    fixture.write(
        CARGO_RULE_OWNER,
        "const CARGO_TEST_BINARY_ENVIRONMENT: &str = \"CARGO_BIN_EXE_\";\n\
         const BAD: Option<&str> = option_env!(\"CARGO_BIN_EXE_yolo-host\");\n",
    );
    fixture.write(
        CARGO_TEST_OWNER,
        "let fixture = \"option_env!(\\\"CARGO_BIN_EXE_yolo-host\\\")\";\n\
         const BAD: Option<&str> = option_env!(\"CARGO_BIN_EXE_yolo-host\");\n",
    );
    let result = messages(&fixture);
    assert_eq!(result.len(), 2);
    assert!(result.iter().all(|entry| entry.contains("Bazel inputs")));
}

#[test]
fn permits_bazel_runfile_and_injected_binary_discovery() {
    let fixture = Fixture::new();
    fixture.write(
        "integration_tests/runtime/resilience.rs",
        "let host = bazel_runfile(HOST_BINARY_RUNFILE);\n\
         let attach = std::env::var_os(BAZEL_ATTACH_BINARY_ENV);\n",
    );
    assert!(messages(&fixture).is_empty());
}
