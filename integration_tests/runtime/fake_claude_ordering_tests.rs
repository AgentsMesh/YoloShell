use crate::fake_claude_authority_contract::{ActivityState, ClaudeHookEndpoint, HookDisposition};
use crate::fake_claude_authority_tests::materialize;
use crate::fake_claude_constants::DATA_HOME_DIRECTORY;
use crate::fake_claude_reference_endpoint::ReferenceEndpoint;
use crate::fake_claude_scenarios::lifecycle;

const PROJECT_DIRECTORY: &str = "project";
const STATE_FILE_NAME: &str = "authority.json";
const STARTUP_FRAME_INDEX: usize = 0;
const PREMATURE_CLEAR_FRAME_INDEX: usize = 4;

#[test]
fn premature_clear_start_invalidates_runtime_authority() {
    let directory = tempfile::tempdir().unwrap();
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let scenario = lifecycle(&directory.path().join(PROJECT_DIRECTORY), &data_home);
    materialize(&scenario.frames);
    let mut endpoint =
        ReferenceEndpoint::open(&directory.path().join(STATE_FILE_NAME), &data_home).unwrap();

    assert_eq!(
        endpoint
            .apply(&scenario.frames[STARTUP_FRAME_INDEX])
            .unwrap(),
        HookDisposition::Applied
    );
    assert_eq!(
        endpoint
            .apply(&scenario.frames[PREMATURE_CLEAR_FRAME_INDEX])
            .unwrap(),
        HookDisposition::Invalidated
    );

    let view = endpoint.view().unwrap();
    assert_eq!(view.activity, ActivityState::Unknown);
    assert!(!view.runtime_active);
}
