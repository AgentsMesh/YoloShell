use super::super::factory::{
    RUNTIME_INSTALLATION_ERROR_MAX_CHARACTERS, RUNTIME_INSTALLATION_LOSS_CONTEXT,
    classify_installation_error,
};
use crate::remote::{RemoteError, RemoteFilesystemOperation};

const DETAIL_OVERFLOW_CHARACTERS: usize = 1;
const EXPECTED_SEPARATOR_CHARACTERS: usize = 2;
const FAILURE_CHARACTER: char = 'x';
const POLICY_FAILURE_DETAIL: &str = "version";
const FILESYSTEM_FAILURE_DETAIL: &str = "permission denied";
const FILESYSTEM_FAILURE_PATH: &str = "/remote/yolo";

#[test]
fn installation_transport_detail_is_preserved_and_bounded() {
    let detail: String = std::iter::repeat_n(
        FAILURE_CHARACTER,
        RUNTIME_INSTALLATION_ERROR_MAX_CHARACTERS + DETAIL_OVERFLOW_CHARACTERS,
    )
    .collect();

    let error = classify_installation_error(RemoteError::transport(detail));
    let RemoteError::RuntimeDeployment(message) = error else {
        panic!("installation transport loss must become a deployment failure");
    };
    assert!(message.starts_with(RUNTIME_INSTALLATION_LOSS_CONTEXT));
    assert_eq!(
        message.chars().count(),
        RUNTIME_INSTALLATION_LOSS_CONTEXT.chars().count()
            + EXPECTED_SEPARATOR_CHARACTERS
            + RUNTIME_INSTALLATION_ERROR_MAX_CHARACTERS
    );
}

#[test]
fn installation_policy_failure_keeps_its_typed_error() {
    let error = classify_installation_error(RemoteError::RuntimeIncompatible(
        POLICY_FAILURE_DETAIL.into(),
    ));

    assert!(matches!(error, RemoteError::RuntimeIncompatible(_)));
}

#[test]
fn installation_filesystem_failure_is_not_wrapped_as_transport_interruption() {
    let primary = RemoteError::RemoteFilesystem {
        operation: RemoteFilesystemOperation::SetPermissions,
        path: FILESYSTEM_FAILURE_PATH.into(),
        detail: FILESYSTEM_FAILURE_DETAIL.into(),
    };
    let error = classify_installation_error(RemoteError::LifecycleSettlement {
        primary: Box::new(primary),
        settlement: Box::new(RemoteError::transport(FILESYSTEM_FAILURE_DETAIL)),
    });

    let RemoteError::LifecycleSettlement { primary, .. } = error else {
        panic!("filesystem settlement failure must preserve both errors")
    };
    let RemoteError::RemoteFilesystem {
        operation, path, ..
    } = *primary
    else {
        panic!("filesystem failure must remain the primary cause")
    };
    assert_eq!(operation, RemoteFilesystemOperation::SetPermissions);
    assert_eq!(path, std::path::Path::new(FILESYSTEM_FAILURE_PATH));
}
