use super::fixture::Fixture;
use super::process_signal_test_support::messages;

const FUTURE_PATH: &str = "core/crates/runtime/engine/process/src/future_shell.rs";
const EXECUTABLE_POLICY: &str = "process signal executable literal is forbidden";

#[test]
fn rejects_signal_commands_in_shell_literals_and_control_lists() {
    for literal in [
        r#""kill -9 42""#,
        r#""echo ready; /usr/bin/pkill agent""#,
        r#""true && killall agent""#,
        r#""sh -c 'kill 42'""#,
        r#""bash -c 'echo ready || pkill agent'""#,
        r#""bash -lc 'kill 42'""#,
        r#""/bin/zsh -c 'kill 42'""#,
        r#""dash -c 'pkill agent'""#,
        r#""ksh -c 'killall agent'""#,
        r#""env zsh -c 'kill 42'""#,
        r#""env -iS 'zsh -c \"kill 42\"'""#,
        r#""env -S'zsh -c \"kill 42\"'""#,
        r#""env -P /bin kill 42""#,
        r#""echo $(kill 42)""#,
        r#""echo `pkill agent`""#,
        r#""command -p kill 42""#,
        r#""env --chdir /tmp kill 42""#,
        r#""exec -a worker kill 42""#,
        r#""nice -n 5 kill 42""#,
        r#""sudo -u root /bin/kill 42""#,
        r#""sudo --user root /bin/kill 42""#,
        r#""eval 'kill 42'""#,
        r#""alias stop=kill; stop 42""#,
        r#""env TOOL=/bin/kill $TOOL 42""#,
    ] {
        let fixture = Fixture::new();
        fixture.write(FUTURE_PATH, &format!("const SCRIPT: &str = {literal};\n"));
        assert!(
            contains(&messages(&fixture), EXECUTABLE_POLICY),
            "{literal}"
        );
    }
}

#[test]
fn accepts_nonexecuting_mentions_and_raw_escape_text() {
    let fixture = Fixture::new();
    fixture.write(
        FUTURE_PATH,
        r##"
const WORD: &str = "skill";
const PROSE: &str = "echo kill";
const NESTED: &str = "sh -c 'echo kill'";
const RAW: &str = r"ki\x6c\x6c";
const QUERY: &str = "command -v kill";
"##,
    );
    assert!(messages(&fixture).is_empty());
}

fn contains(messages: &[String], expected: &str) -> bool {
    messages.iter().any(|message| message.contains(expected))
}
