use super::process_supervision_tests::Fixture;
use crate::constants::{BAZEL_REAL_ENVIRONMENT, HOME_ENVIRONMENT};
use yolo_process::PROCESS_DISCOVERY_CAPABILITY_ENV;

const ENV_EXECUTABLE: &str = "/usr/bin/env";
const ENV_DESCRIPTION: &str = "sanitized environment probe";
const ENV_ASSIGNMENT_SEPARATOR: char = '=';

#[test]
fn sanitized_subprocess_drops_ambient_inputs() {
    let fixture = Fixture::new();
    let output = fixture
        .command(ENV_EXECUTABLE, Vec::new(), ENV_DESCRIPTION)
        .execute()
        .unwrap();
    let environment = String::from_utf8(output.stdout).unwrap();
    let keys = environment
        .lines()
        .filter_map(|entry| entry.split_once(ENV_ASSIGNMENT_SEPARATOR))
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    assert!(!keys.iter().any(|key| *key == BAZEL_REAL_ENVIRONMENT));
    assert!(keys.contains(&HOME_ENVIRONMENT));
    assert!(keys.contains(&PROCESS_DISCOVERY_CAPABILITY_ENV));
}
