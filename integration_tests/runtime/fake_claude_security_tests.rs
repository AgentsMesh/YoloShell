use crate::fake_claude_authority_contract::{ClaudeHookEndpoint, HookDisposition};
use crate::fake_claude_authority_tests::{followup, materialize};
use crate::fake_claude_constants::DATA_HOME_DIRECTORY;
use crate::fake_claude_reference_endpoint::ReferenceEndpoint;
use crate::fake_claude_scenarios::resume;
use std::fs;

const STATE_FILE_NAME: &str = "authority.json";
const SAVED_TRANSCRIPT_NAME: &str = "saved-transcript.jsonl";
const SYMLINK_TARGET_NAME: &str = "symlink-target.jsonl";
const REPLACEMENT_CONTENTS: &[u8] = b"replacement\n";
const REUSED_START_INCREMENT: u64 = 1;
const FOREIGN_TOKEN: &str = "faaaaaaa-ffff-4fff-8fff-ffffffffffff";
const FOREIGN_EPOCH: &str = "eaaaaaaa-eeee-4eee-8eee-eeeeeeeeeeee";
const INITIAL_FRAME_INDEX: usize = 0;
const EXTERNAL_EVIDENCE_DIRECTORY: &str = "external-evidence";

#[test]
fn pid_reuse_token_epoch_and_ancestry_changes_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let scenario = resume(directory.path(), &data_home);
    materialize(&scenario.frames);
    let mut endpoint =
        ReferenceEndpoint::open(&directory.path().join(STATE_FILE_NAME), &data_home).unwrap();
    endpoint
        .apply(&scenario.frames[INITIAL_FRAME_INDEX])
        .unwrap();
    let expected = endpoint.view();
    let base = followup(&scenario.frames[INITIAL_FRAME_INDEX]);

    let mut reused_pid = base.clone();
    reused_pid.authority.agent.start_time += REUSED_START_INCREMENT;
    let mut foreign_token = base.clone();
    foreign_token.authority.hook_token = FOREIGN_TOKEN.into();
    let mut foreign_epoch = base.clone();
    foreign_epoch.authority.runtime_epoch = FOREIGN_EPOCH.into();
    let mut broken_ancestry = base;
    broken_ancestry.authority.hook.parent_pid = Some(broken_ancestry.authority.shell.pid);
    for frame in [reused_pid, foreign_token, foreign_epoch, broken_ancestry] {
        assert_eq!(endpoint.apply(&frame).unwrap(), HookDisposition::Rejected);
        assert_eq!(endpoint.view(), expected);
    }
}

#[test]
fn transcript_symlink_and_inode_swap_cannot_replace_durable_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let scenario = resume(directory.path(), &data_home);
    let transcript = &scenario.frames[INITIAL_FRAME_INDEX]
        .input
        .common()
        .transcript_path;
    fs::create_dir_all(transcript.parent().unwrap()).unwrap();
    let target = directory.path().join(SYMLINK_TARGET_NAME);
    fs::write(&target, REPLACEMENT_CONTENTS).unwrap();
    std::os::unix::fs::symlink(&target, transcript).unwrap();
    let mut endpoint =
        ReferenceEndpoint::open(&directory.path().join(STATE_FILE_NAME), &data_home).unwrap();
    assert_eq!(
        endpoint
            .apply(&scenario.frames[INITIAL_FRAME_INDEX])
            .unwrap(),
        HookDisposition::Rejected
    );

    fs::remove_file(transcript).unwrap();
    fs::write(transcript, REPLACEMENT_CONTENTS).unwrap();
    assert_eq!(
        endpoint
            .apply(&scenario.frames[INITIAL_FRAME_INDEX])
            .unwrap(),
        HookDisposition::Applied
    );
    let accepted = endpoint.view();
    fs::rename(transcript, directory.path().join(SAVED_TRANSCRIPT_NAME)).unwrap();
    fs::write(transcript, REPLACEMENT_CONTENTS).unwrap();
    assert_eq!(
        endpoint
            .apply(&followup(&scenario.frames[INITIAL_FRAME_INDEX]))
            .unwrap(),
        HookDisposition::Rejected
    );
    assert_eq!(endpoint.view(), accepted);
}

#[test]
fn symlinked_transcript_ancestor_is_rejected_component_by_component() {
    let directory = tempfile::tempdir().unwrap();
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let scenario = resume(directory.path(), &data_home);
    let transcript = &scenario.frames[INITIAL_FRAME_INDEX]
        .input
        .common()
        .transcript_path;
    let evidence = data_home.join(yolo_runtime::agent::claude::constants::EVIDENCE_DIR);
    let relative = transcript.strip_prefix(&evidence).unwrap();
    let external = directory.path().join(EXTERNAL_EVIDENCE_DIRECTORY);
    let external_transcript = external.join(relative);
    fs::create_dir_all(external_transcript.parent().unwrap()).unwrap();
    fs::write(&external_transcript, REPLACEMENT_CONTENTS).unwrap();
    fs::create_dir_all(&data_home).unwrap();
    std::os::unix::fs::symlink(&external, &evidence).unwrap();
    let mut endpoint =
        ReferenceEndpoint::open(&directory.path().join(STATE_FILE_NAME), &data_home).unwrap();
    assert_eq!(
        endpoint
            .apply(&scenario.frames[INITIAL_FRAME_INDEX])
            .unwrap(),
        HookDisposition::Rejected
    );
}
