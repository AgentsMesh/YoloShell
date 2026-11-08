use super::*;
use std::collections::HashSet;

const TEST_RUNTIME_EPOCH: &str = "runtime epoch with spaces";
const TEST_SESSION_ID: &str = "session 'quoted'\nsecond-line";
const CONCURRENT_INVOCATION_COUNT: usize = 16;
const EXTRA_ARGUMENT: &str = "unexpected";
const ARGUMENT_WITH_SPACES: &str = "two words";
const CONCURRENT_EPOCH_PREFIX: &str = "epoch";
const INVALID_NOT_JSON: &str = "not-json";
const INVALID_ARRAY: &str = "[]";
const INVALID_EXTRA_FIELD: &str = r#"{"runtime_epoch":"epoch","arguments":[],"extra":true}"#;
const INVALID_RUNTIME_EPOCH: &str = r#"{"runtime_epoch":1,"arguments":[]}"#;
const INVALID_ARGUMENT: &str = r#"{"runtime_epoch":"epoch","arguments":[null]}"#;

#[test]
fn invocation_jsonl_round_trip_preserves_exact_argument_boundaries() {
    let arguments = vec![
        String::new(),
        ARGUMENT_WITH_SPACES.to_owned(),
        TEST_SESSION_ID.to_owned(),
    ];
    let encoded = invocation::encode(TEST_RUNTIME_EPOCH, &arguments).unwrap();
    assert_eq!(
        encoded.last(),
        Some(&fake_agent_constants::INVOCATION_LINE_ENDING)
    );
    let line_ending = [fake_agent_constants::INVOCATION_LINE_ENDING];
    let line = std::str::from_utf8(encoded.strip_suffix(&line_ending).unwrap()).unwrap();
    assert_eq!(
        invocation::decode(line).unwrap(),
        invocation::Invocation {
            runtime_epoch: TEST_RUNTIME_EPOCH.to_owned(),
            arguments,
        }
    );
}

#[test]
fn invocation_parser_rejects_non_exact_or_non_string_records() {
    let malformed = [
        INVALID_NOT_JSON,
        INVALID_ARRAY,
        INVALID_EXTRA_FIELD,
        INVALID_RUNTIME_EPOCH,
        INVALID_ARGUMENT,
    ];
    for value in malformed {
        assert!(invocation::decode(value).is_err(), "accepted {value}");
    }
}

#[test]
fn session_selection_requires_the_exact_provider_resume_argv() {
    for provider in [FakeAgentProvider::Codex, FakeAgentProvider::Claude] {
        assert_eq!(
            session_id(provider, &[], TEST_RUNTIME_EPOCH),
            Ok(TEST_RUNTIME_EPOCH)
        );
        let mut resume = vec![provider.resume_argument().to_owned()];
        if provider == FakeAgentProvider::Codex {
            resume.extend([
                yolo_runtime::agent::codex::constants::CWD_ARGUMENT.to_owned(),
                std::env::current_dir().unwrap().display().to_string(),
            ]);
        }
        resume.push(TEST_SESSION_ID.to_owned());
        assert_eq!(
            session_id(provider, &resume, TEST_RUNTIME_EPOCH),
            Ok(TEST_SESSION_ID)
        );
        assert_eq!(
            session_id(
                provider,
                &[provider.resume_argument().to_owned()],
                TEST_RUNTIME_EPOCH,
            ),
            Err(RESUME_SESSION_ID_MISSING)
        );
        let mut extra = resume;
        extra.push(EXTRA_ARGUMENT.to_owned());
        assert_eq!(
            session_id(provider, &extra, TEST_RUNTIME_EPOCH),
            Err(INVOCATION_ARGUMENTS_UNSUPPORTED)
        );
        assert_eq!(
            session_id(provider, &[EXTRA_ARGUMENT.to_owned()], TEST_RUNTIME_EPOCH),
            Err(INVOCATION_ARGUMENTS_UNSUPPORTED)
        );
    }
}

#[test]
fn concurrent_invocation_appends_remain_individually_parseable() {
    let directory = tempfile::tempdir().unwrap();
    let handles: Vec<_> = (0..CONCURRENT_INVOCATION_COUNT)
        .map(|index| {
            let path = directory.path().to_path_buf();
            std::thread::spawn(move || {
                let epoch = format!("{CONCURRENT_EPOCH_PREFIX} {index}");
                record_invocation(&path, &epoch, &[TEST_SESSION_ID.to_owned()]).unwrap();
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let contents = fs::read_to_string(
        directory
            .path()
            .join(fake_agent_constants::INVOCATION_LOG_FILE),
    )
    .unwrap();
    let records: Vec<_> = contents
        .lines()
        .map(|line| invocation::decode(line).unwrap())
        .collect();
    assert_eq!(records.len(), CONCURRENT_INVOCATION_COUNT);
    assert!(
        records
            .iter()
            .all(|record| record.arguments == [TEST_SESSION_ID])
    );
    let epochs: HashSet<_> = records
        .into_iter()
        .map(|record| record.runtime_epoch)
        .collect();
    assert_eq!(epochs.len(), CONCURRENT_INVOCATION_COUNT);
}
