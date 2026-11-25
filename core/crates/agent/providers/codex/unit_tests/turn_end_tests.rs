use crate::codex::constants as c;
use crate::codex::{CodexAdapter, TurnEnd, TurnEndKind};
use serde_json::json;
use std::path::{Path, PathBuf};

const TURN_A: &str = "turn-a";
const TURN_B: &str = "turn-b";

fn write_rollout(home: &Path, lines: &[serde_json::Value]) -> PathBuf {
    let directory = home.join("sessions/2026/10/28");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("rollout-test.jsonl");
    let body = lines
        .iter()
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    std::fs::write(&path, body).unwrap();
    path
}

fn event(kind: &str, turn_id: &str) -> serde_json::Value {
    json!({
        (c::RECORD_TYPE_KEY): c::EVENT_MESSAGE_TYPE,
        (c::PAYLOAD_KEY): {
            (c::EVENT_KIND_KEY): kind,
            (c::TURN_ID_KEY): turn_id,
        },
    })
}

fn read(home: &Path, path: &Path) -> Option<TurnEnd> {
    CodexAdapter.read_turn_end(path, home).unwrap()
}

#[test]
fn latest_completed_turn_is_reported() {
    let home = tempfile::tempdir().unwrap();
    let path = write_rollout(
        home.path(),
        &[
            event(c::TASK_STARTED_EVENT, TURN_A),
            event(c::TOKEN_COUNT_EVENT, TURN_A),
            event(c::TASK_COMPLETE_EVENT, TURN_A),
        ],
    );
    assert_eq!(
        read(home.path(), &path),
        Some(TurnEnd {
            turn_id: TURN_A.into(),
            kind: TurnEndKind::Completed,
        })
    );
}

#[test]
fn interrupted_turn_is_reported() {
    let home = tempfile::tempdir().unwrap();
    let path = write_rollout(
        home.path(),
        &[
            event(c::TASK_STARTED_EVENT, TURN_A),
            event(c::TURN_ABORTED_EVENT, TURN_A),
        ],
    );
    assert_eq!(
        read(home.path(), &path),
        Some(TurnEnd {
            turn_id: TURN_A.into(),
            kind: TurnEndKind::Aborted,
        })
    );
}

#[test]
fn a_later_started_turn_supersedes_the_previous_completion() {
    let home = tempfile::tempdir().unwrap();
    let path = write_rollout(
        home.path(),
        &[
            event(c::TASK_STARTED_EVENT, TURN_A),
            event(c::TASK_COMPLETE_EVENT, TURN_A),
            event(c::TASK_STARTED_EVENT, TURN_B),
        ],
    );
    assert_eq!(read(home.path(), &path), None);
}

#[test]
fn trailing_activity_does_not_erase_a_completed_turn() {
    let home = tempfile::tempdir().unwrap();
    let path = write_rollout(
        home.path(),
        &[
            event(c::TASK_STARTED_EVENT, TURN_A),
            event(c::TASK_COMPLETE_EVENT, TURN_A),
            event(c::TOKEN_COUNT_EVENT, TURN_A),
        ],
    );
    assert_eq!(
        read(home.path(), &path),
        Some(TurnEnd {
            turn_id: TURN_A.into(),
            kind: TurnEndKind::Completed,
        })
    );
}

#[test]
fn records_without_a_turn_id_are_ignored() {
    let home = tempfile::tempdir().unwrap();
    let path = write_rollout(
        home.path(),
        &[
            json!({
                (c::RECORD_TYPE_KEY): c::SESSION_META_TYPE,
                (c::PAYLOAD_KEY): {(c::SESSION_ID_KEY): "session"},
            }),
            json!({
                (c::RECORD_TYPE_KEY): c::EVENT_MESSAGE_TYPE,
                (c::PAYLOAD_KEY): {(c::EVENT_KIND_KEY): c::TASK_STARTED_EVENT},
            }),
            event(c::TASK_COMPLETE_EVENT, TURN_A),
        ],
    );
    assert_eq!(
        read(home.path(), &path),
        Some(TurnEnd {
            turn_id: TURN_A.into(),
            kind: TurnEndKind::Completed,
        })
    );
}

#[test]
fn subagent_records_do_not_replace_the_main_turn() {
    let home = tempfile::tempdir().unwrap();
    let path = write_rollout(
        home.path(),
        &[
            event(c::TASK_STARTED_EVENT, TURN_A),
            event(c::TASK_COMPLETE_EVENT, TURN_A),
            json!({
                (c::RECORD_TYPE_KEY): c::EVENT_MESSAGE_TYPE,
                (c::PAYLOAD_KEY): {
                    (c::EVENT_KIND_KEY): c::TASK_STARTED_EVENT,
                    (c::TURN_ID_KEY): TURN_B,
                    (c::AGENT_ID_KEY): "child",
                },
            }),
        ],
    );
    assert_eq!(
        read(home.path(), &path),
        Some(TurnEnd {
            turn_id: TURN_A.into(),
            kind: TurnEndKind::Completed,
        })
    );
}

#[test]
fn a_tail_window_starting_mid_line_still_finds_the_terminal_record() {
    let home = tempfile::tempdir().unwrap();
    let filler = "x".repeat(c::TURN_TAIL_BYTES as usize);
    let path = write_rollout(
        home.path(),
        &[
            json!({
                (c::RECORD_TYPE_KEY): c::EVENT_MESSAGE_TYPE,
                (c::PAYLOAD_KEY): {
                    (c::EVENT_KIND_KEY): c::TOKEN_COUNT_EVENT,
                    "text": filler,
                },
            }),
            event(c::TASK_COMPLETE_EVENT, TURN_A),
        ],
    );
    assert_eq!(
        read(home.path(), &path),
        Some(TurnEnd {
            turn_id: TURN_A.into(),
            kind: TurnEndKind::Completed,
        })
    );
}
