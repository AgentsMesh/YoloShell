use super::fixture::Fixture;
use super::process_signal_test_support::messages;
const DETACHED_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/process/src/detached_child_custody.rs";
const UNIDENTIFIED_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/process/src/unidentified_pty_child_custody.rs";
const BRIDGE_CHILD_PATH: &str = "core/crates/application/host/cli/src/bridge_child.rs";
const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_child.rs";
const KILL_POLICY: &str = "direct child termination is outside its exact process custody owner";
const EXECUTABLE_POLICY: &str = "process signal executable literal is forbidden";

#[test]
fn accepts_only_the_exact_production_custody_sites() {
    let fixture = Fixture::new();
    fixture.write(
        DETACHED_CUSTODY_PATH,
        "impl DetachedChildCustody { fn request_direct_termination(&mut self) { ::std::process::Child::kill(&mut self.child); } }\n",
    );
    fixture.write(
        UNIDENTIFIED_CUSTODY_PATH,
        "impl UnidentifiedPtyChildCustody { fn request_direct_termination(&mut self) { self.child.kill(); } }\n",
    );
    fixture.write(
        BRIDGE_CHILD_PATH,
        "impl Drop for SpawnedHost { fn drop(&mut self) { ::std::process::Child::kill(&mut child); } }\n",
    );
    assert!(
        messages(&fixture)
            .iter()
            .all(|message| !message.contains(KILL_POLICY))
    );
}

#[test]
fn renamed_receivers_and_moved_calls_are_rejected() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        "fn terminate() {\n renamed_child.kill();\n self.child.kill();\n}\n",
    );
    assert_eq!(count(&messages(&fixture), KILL_POLICY), 2);
}

#[test]
fn exact_files_still_require_the_exact_owner_and_function() {
    let fixture = Fixture::new();
    fixture.write(
        DETACHED_CUSTODY_PATH,
        "impl DetachedChildCustody { fn moved(&mut self) { self.child.kill(); } }\n",
    );
    fixture.write(
        UNIDENTIFIED_CUSTODY_PATH,
        "impl UnidentifiedPtyChildCustody { fn moved(&mut self) { self.child.kill(); } }\n",
    );
    fixture.write(
        BRIDGE_CHILD_PATH,
        "impl Drop for OtherHost { fn drop(&mut self) { child.kill(); } }\n",
    );
    assert_eq!(count(&messages(&fixture), KILL_POLICY), 3);
}

#[test]
fn qualified_calls_and_function_references_are_rejected() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        r#"
fn terminate() {
    Child::kill(&mut child);
    let terminate = Child::kill;
    nix::sys::signal::kill(pid, signal);
}
"#,
    );
    assert_eq!(count(&messages(&fixture), KILL_POLICY), 3);
}

#[test]
fn direct_grouped_and_aliased_imports_are_rejected() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        r#"
use nix::sys::signal::kill;
use nix::sys::signal::{kill as terminate};
use other::kill as stop;
"#,
    );
    assert_eq!(count(&messages(&fixture), KILL_POLICY), 3);
}

#[test]
fn signal_executable_basenames_are_rejected_even_through_constants() {
    let fixture = Fixture::new();
    let source = format!(
        r#"
const TOOL: &str = "/opt/bin/kill";
fn launch() {{
    Command::new("/bin/kill");
    Command::new("/usr/bin/pkill");
    Command::new("{}");
    let alias = TOOL;
    Command::new(alias);
}}
"#,
        yolo_architecture_policy_contracts::process_signal::KILLALL_EXECUTABLE
    );
    fixture.write(FUTURE_PATH, source);
    assert_eq!(count(&messages(&fixture), EXECUTABLE_POLICY), 4);
}

#[test]
fn test_sources_and_unit_test_functions_may_terminate_children() {
    let test_source = Fixture::new();
    test_source.write(
        "core/crates/runtime/engine/process/src/future_child_tests.rs",
        "fn helper() { child.kill(); Child::kill(&mut child); Command::new(\"/bin/kill\"); }\n",
    );
    assert!(messages(&test_source).is_empty());

    let guarded = Fixture::new();
    let source = format!(
        "#[cfg(all(unix, test))]\nfn helper() {{ child.kill(); Command::new(\"{}\"); }}\n",
        yolo_architecture_policy_contracts::process_signal::KILLALL_EXECUTABLE
    );
    guarded.write(FUTURE_PATH, source);
    assert!(messages(&guarded).is_empty());
}

#[test]
fn exact_cfg_test_integration_holder_is_a_test_source() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/runtime/engine/process/src/pty_session_linux_integration_holder.rs",
        "impl Drop for HolderFixture { fn drop(&mut self) { root.kill(); } }\n",
    );
    assert!(messages(&fixture).is_empty());

    let moved = Fixture::new();
    moved.write(
        FUTURE_PATH,
        "impl Drop for HolderFixture { fn drop(&mut self) { root.kill(); } }\n",
    );
    assert_eq!(count(&messages(&moved), KILL_POLICY), 1);
}

#[test]
fn imported_kill_modules_are_allowed_only_as_namespaces() {
    let namespace = Fixture::new();
    namespace.write(
        FUTURE_PATH,
        "use crate::typed::{freeze, kill};\nfn commit() { kill::signal(); }\n",
    );
    assert!(messages(&namespace).is_empty());

    let callable = Fixture::new();
    callable.write(
        FUTURE_PATH,
        "use nix::sys::signal::kill;\nfn bypass() { kill(pid, signal); }\n",
    );
    assert_eq!(count(&messages(&callable), KILL_POLICY), 1);
}

#[test]
fn adversarial_feature_does_not_grant_production_kill_authority() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        r#"
#[cfg(feature = "adversarial-test-hooks")]
fn helper() { child.kill(); Command::new("/bin/kill"); }
"#,
    );
    let result = messages(&fixture);
    assert_eq!(count(&result, KILL_POLICY), 1);
    assert_eq!(count(&result, EXECUTABLE_POLICY), 1);
}

#[test]
fn comments_unqualified_functions_and_non_signal_paths_do_not_match() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        r#"
// renamed_child.kill(); Child::kill(&mut child);
/* use nix::sys::signal::kill as terminate; */
fn kill() {}
fn helper() { kill(); Command::new("/bin/skill"); }
"#,
    );
    assert!(messages(&fixture).is_empty());
}

fn count(messages: &[String], needle: &str) -> usize {
    messages
        .iter()
        .filter(|value| value.contains(needle))
        .count()
}
