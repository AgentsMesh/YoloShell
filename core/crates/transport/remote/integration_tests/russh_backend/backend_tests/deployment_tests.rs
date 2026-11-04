use super::constants::*;
use super::deployment_support::{artifact, deployment_fixture, runtime};
use super::memory_fs::FailurePoint;
use crate::remote::constants::RUNTIME_EXECUTABLE_MODE;
use crate::remote::{RuntimeDeploymentResult, RuntimeInstaller};

#[tokio::test]
async fn runtime_install_is_content_addressed_current_and_upgradeable() {
    let fixture = deployment_fixture(INITIAL_RUNTIME_VERSION).await;
    let runtime = runtime();
    let first = artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL);
    let installer = RuntimeInstaller;
    assert_eq!(
        installer
            .ensure(fixture.connection.as_ref(), &runtime, &first)
            .await
            .unwrap(),
        RuntimeDeploymentResult::Installed
    );
    let first_target = fixture.filesystem.target(MEMORY_RUNTIME).unwrap();
    assert_eq!(
        fixture.filesystem.permissions(&first_target),
        Some(RUNTIME_EXECUTABLE_MODE)
    );
    let writes = fixture.filesystem.writes();
    let activations = fixture.filesystem.activations();
    assert_eq!(
        installer
            .ensure(fixture.connection.as_ref(), &runtime, &first)
            .await
            .unwrap(),
        RuntimeDeploymentResult::Current
    );
    assert_eq!(fixture.filesystem.writes(), writes);
    assert_eq!(fixture.filesystem.activations(), activations);

    fixture.filesystem.set_version(UPGRADED_RUNTIME_VERSION);
    let upgraded = artifact(UPGRADED_RUNTIME_VERSION, UPGRADED_BINARY_FILL);
    assert_eq!(
        installer
            .ensure(fixture.connection.as_ref(), &runtime, &upgraded)
            .await
            .unwrap(),
        RuntimeDeploymentResult::Upgraded
    );
    assert_ne!(
        fixture.filesystem.target(MEMORY_RUNTIME),
        Some(first_target.clone())
    );
    assert!(fixture.filesystem.contains_file(&first_target));
    assert_eq!(fixture.filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn concurrent_installs_converge_on_the_same_release() {
    let fixture = deployment_fixture(INITIAL_RUNTIME_VERSION).await;
    let runtime = runtime();
    let artifact = artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL);
    let first = RuntimeInstaller.ensure(fixture.connection.as_ref(), &runtime, &artifact);
    let second = RuntimeInstaller.ensure(fixture.connection.as_ref(), &runtime, &artifact);
    let (first, second) = tokio::join!(first, second);
    assert!(first.is_ok());
    assert!(second.is_ok());
    let target = fixture.filesystem.target(MEMORY_RUNTIME).unwrap();
    assert!(target.ends_with(artifact.sha256()));
    assert!(fixture.filesystem.contains_file(&target));
    assert_eq!(fixture.filesystem.temporary_entries(), 0);
}

#[tokio::test]
async fn deployment_failures_preserve_the_previous_runtime() {
    let runtime = runtime();
    let first = artifact(INITIAL_RUNTIME_VERSION, INITIAL_BINARY_FILL);
    let upgraded = artifact(UPGRADED_RUNTIME_VERSION, UPGRADED_BINARY_FILL);
    for failure in [
        FailurePoint::CorruptUpload,
        FailurePoint::Permission,
        FailurePoint::ReleaseRename,
        FailurePoint::ActivationRename,
        FailurePoint::SftpTransport,
    ] {
        let fixture = deployment_fixture(INITIAL_RUNTIME_VERSION).await;
        RuntimeInstaller
            .ensure(fixture.connection.as_ref(), &runtime, &first)
            .await
            .unwrap();
        let stable = fixture.filesystem.target(MEMORY_RUNTIME).unwrap();
        fixture.filesystem.set_version(UPGRADED_RUNTIME_VERSION);
        fixture.filesystem.fail_once(failure);
        assert!(
            RuntimeInstaller
                .ensure(fixture.connection.as_ref(), &runtime, &upgraded)
                .await
                .is_err(),
            "deployment unexpectedly succeeded for {failure:?}"
        );
        assert_eq!(
            fixture.filesystem.target(MEMORY_RUNTIME),
            Some(stable.clone())
        );
        assert!(fixture.filesystem.contains_file(&stable));
        assert_eq!(fixture.filesystem.temporary_entries(), 0);
    }
}
