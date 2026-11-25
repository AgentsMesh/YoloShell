use super::*;
use crate::fixture_constants as fixture;
use std::os::unix::ffi::OsStringExt;
use yolo_remote_broker::BrokerLaunchDeadline;

const CONFIG_PREFIX: &[u8] = b"/private/config-";
const STATE_PREFIX: &[u8] = b"/private/state-";
const ACTIVATION_DESCRIPTOR: i32 = 9;

#[test]
fn typed_launch_preserves_opaque_paths_deadline_and_explicit_remote_role() {
    let opaque = |prefix: &[u8]| {
        let mut path = prefix.to_vec();
        path.push(fixture::NON_UTF8_PATH_BYTE);
        std::path::PathBuf::from(OsString::from_vec(path))
    };
    let launch = ChildLaunch::new(
        opaque(CONFIG_PREFIX),
        opaque(STATE_PREFIX),
        fixture::TEST_CONFIG_REVISION.into(),
        fixture::TEST_STATE_ANCHOR.parse().unwrap(),
        uuid::Uuid::nil().to_string().parse().unwrap(),
        BrokerLaunchDeadline::capture(tokio::time::Instant::now() + fixture::START_TIMEOUT)
            .unwrap(),
    )
    .unwrap();
    for source in [CredentialSource::None, CredentialSource::PrivateStdin] {
        let command = build(
            constants::SUPERVISED_SERVE_COMMAND,
            &launch,
            source,
            Some(ACTIVATION_DESCRIPTOR),
        )
        .unwrap();
        let role = yolo_cli::ProcessRole::Remote
            .arguments()
            .map(OsString::from);
        assert!(command.arguments.starts_with(&role));
        assert_eq!(
            command.arguments[role.len()],
            constants::SUPERVISED_SERVE_COMMAND
        );
        for (flag, value) in [
            (
                constants::CONFIG_ARGUMENT,
                launch.config().as_os_str().to_owned(),
            ),
            (
                constants::STATE_DIRECTORY_ARGUMENT,
                launch.state_dir().as_os_str().to_owned(),
            ),
            (
                constants::EXPECTED_STARTUP_DEADLINE_ARGUMENT,
                launch.startup_deadline().to_string().into(),
            ),
            (
                constants::SUPERVISION_ACTIVATION_FD_ARGUMENT,
                ACTIVATION_DESCRIPTOR.to_string().into(),
            ),
        ] {
            assert!(
                command
                    .arguments
                    .windows(2)
                    .any(|pair| pair[0] == flag && pair[1] == value)
            );
        }
        assert_eq!(
            command
                .arguments
                .iter()
                .filter(|argument| *argument == constants::IDENTITY_PASSPHRASES_STDIN_ARGUMENT)
                .count(),
            usize::from(matches!(source, CredentialSource::PrivateStdin))
        );
    }
}
