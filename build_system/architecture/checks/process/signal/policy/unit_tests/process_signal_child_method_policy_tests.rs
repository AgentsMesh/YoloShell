use super::fixture::Fixture;
use super::process_signal_test_support::messages;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_child_api.rs";
const LIFECYCLE_POLICY: &str = "implicit child termination is forbidden";

#[test]
fn rejects_implicit_child_termination_apis() {
    for source in [
        "fn bypass(child: &mut Child) { child.start_kill(); }\n",
        "fn bypass(command: &mut Command) { command.kill_on_drop(true); }\n",
        "fn bypass(command: &mut Command, enabled: bool) { command.kill_on_drop(enabled); }\n",
        "fn bypass() { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL); }\n",
        "fn bypass() { libc::syscall(libc::SYS_prctl, libc::PR_SET_PDEATHSIG, 9); }\n",
        "fn bypass(child: &mut Child) { Child::start_kill(child); }\n",
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, source);
        assert!(contains(&messages(&fixture), LIFECYCLE_POLICY), "{source}");
    }
}

#[test]
fn allows_explicitly_disabled_kill_on_drop_and_unrelated_names() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        "fn safe(command: &mut Command) { command.kill_on_drop(false,); Command::kill_on_drop(command, false); command.start(); }\n",
    );
    assert!(messages(&fixture).is_empty());
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
