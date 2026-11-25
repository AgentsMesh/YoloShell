use super::super::*;
use super::support::connection as fake_connection;
use crate::remote::constants::*;

const REMOTE_HOME: &str = "/var/lib/yolo";

#[tokio::test]
async fn every_runtime_probe_failure_fails_before_staging() {
    let cases = [
        output(b"Linux", true, false, Some(0)),
        output(b"Linux", false, true, Some(0)),
        output(b"Linux", false, false, Some(1)),
        output(&[0xff], false, false, Some(0)),
        output(b"Darwin", false, false, Some(0)),
    ];
    for probe in cases {
        let connection = fake_connection(REMOTE_HOME, "boot");
        connection.set_command_outputs([probe]);
        assert!(
            RuntimeInstaller
                .ensure(connection.as_ref(), &runtime(), &artifact())
                .await
                .is_err()
        );
        assert_eq!(connection.purposes(), [ChannelPurpose::Command]);
    }
}

#[tokio::test]
async fn stage_version_and_activation_channels_are_independently_verified() {
    let connection = fake_connection(REMOTE_HOME, "boot");
    connection.set_command_outputs(probe_outputs(b"1.0.0"));
    connection.set_purpose_overrides([
        ChannelPurpose::Command,
        ChannelPurpose::Command,
        ChannelPurpose::Command,
    ]);
    assert!(
        RuntimeInstaller
            .ensure(connection.as_ref(), &runtime(), &artifact())
            .await
            .is_err()
    );

    let connection = fake_connection(REMOTE_HOME, "boot");
    connection.set_command_outputs(probe_outputs(b"wrong"));
    assert!(
        RuntimeInstaller
            .ensure(connection.as_ref(), &runtime(), &artifact())
            .await
            .is_err()
    );

    let connection = fake_connection(REMOTE_HOME, "boot");
    connection.set_command_outputs(probe_outputs(b"1.0.0"));
    connection.set_purpose_overrides([
        ChannelPurpose::Command,
        ChannelPurpose::Command,
        ChannelPurpose::RuntimeDeployment,
        ChannelPurpose::Command,
        ChannelPurpose::Command,
    ]);
    assert!(
        RuntimeInstaller
            .ensure(connection.as_ref(), &runtime(), &artifact())
            .await
            .is_err()
    );
}

fn probe_outputs(version: &[u8]) -> Vec<CommandOutput> {
    vec![
        output(b"Linux", false, false, Some(0)),
        output(
            crate::remote::constants::ARCHITECTURE_X86_64.as_bytes(),
            false,
            false,
            Some(0),
        ),
        output(version, false, false, Some(0)),
    ]
}

fn output(
    stdout: &[u8],
    timed_out: bool,
    truncated: bool,
    exit_status: Option<u32>,
) -> CommandOutput {
    CommandOutput {
        stdout: stdout.into(),
        stderr: Vec::new(),
        exit_status,
        timed_out,
        truncated,
    }
}

fn runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::new("/opt/yolo/yolo", REMOTE_HOME).unwrap()
}

fn artifact() -> RuntimeArtifact {
    let mut bytes = vec![0; ELF_HEADER_BYTES];
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_CLASS_64;
    bytes[ELF_ENDIAN_OFFSET] = ELF_LITTLE_ENDIAN;
    bytes[ELF_VERSION_OFFSET] = ELF_VERSION_CURRENT;
    bytes[ELF_MACHINE_OFFSET..ELF_HEADER_BYTES].copy_from_slice(&ELF_MACHINE_X86_64.to_le_bytes());
    let digest = russh::keys::HashAlg::Sha256.digest(&bytes);
    let sha256 = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    RuntimeArtifact::new("1.0.0", RemoteArchitecture::X86_64, bytes, sha256).unwrap()
}
