use crate::host::tests::*;
use super::*;

use std::cell::RefCell;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

const TEST_IMMEDIATE_SOCKET_TIMEOUT: Duration = Duration::ZERO;
const REPLACEMENT_SOCKET_MODE: u32 = 0o700;

#[tokio::test]
async fn host_socket_probe_timeout_and_indeterminate_error_are_fail_closed() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("host.sock");
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_socket_permissions(&path).unwrap();
    let identity = socket_identity(&path).unwrap();

    let timed_out = bounded_host_socket_probe(
        std::future::pending::<std::io::Result<()>>(),
        TEST_IMMEDIATE_SOCKET_TIMEOUT,
    )
    .await;
    let timeout_error = finish_host_socket_probe(&path, identity, timed_out)
        .err()
        .unwrap();
    assert!(
        timeout_error
            .to_string()
            .contains(HOST_SOCKET_PROBE_TIMEOUT_CONTEXT)
    );

    let denied = std::future::ready(Err::<(), _>(std::io::Error::from(
        std::io::ErrorKind::PermissionDenied,
    )));
    let indeterminate = bounded_host_socket_probe(denied, SOCKET_PROBE_TIMEOUT).await;
    let probe_error = finish_host_socket_probe(&path, identity, indeterminate)
        .err()
        .unwrap();
    assert!(
        probe_error
            .to_string()
            .contains(HOST_SOCKET_PROBE_ERROR_CONTEXT)
    );
    assert_eq!(socket_identity(&path).unwrap(), identity);
    drop(listener);
}

#[test]
fn stale_socket_identity_cannot_authorize_removing_a_replacement() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("host.sock");
    let stale = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_socket_permissions(&path).unwrap();
    let stale_identity = socket_identity(&path).unwrap();
    drop(stale);
    let replacement = RefCell::new(None);

    let error =
        finish_host_socket_probe_with_hook(&path, stale_identity, HostSocketProbe::Refused, || {
            std::fs::remove_file(&path)?;
            let listener = std::os::unix::net::UnixListener::bind(&path)?;
            set_current_socket_permissions(&path)?;
            replacement.replace(Some(listener));
            Ok(())
        })
        .err()
        .unwrap();

    assert!(error.to_string().contains(HOST_SOCKET_CHANGED_CONTEXT));
    assert_ne!(socket_identity(&path).unwrap(), stale_identity);
    drop(replacement);
}

#[test]
fn chmod_rejects_a_socket_replaced_after_bind_identity_capture() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("host.sock");
    let original = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let original_identity = bound_socket_identity(&path).unwrap();
    let replacement = RefCell::new(None);

    let error = set_socket_permissions_with_hook(&path, original_identity, || {
        std::fs::remove_file(&path)?;
        let listener = std::os::unix::net::UnixListener::bind(&path)?;
        std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(REPLACEMENT_SOCKET_MODE),
        )?;
        replacement.replace(Some(listener));
        Ok(())
    })
    .err()
    .unwrap();

    assert!(error.to_string().contains(HOST_SOCKET_CHANGED_CONTEXT));
    assert_ne!(bound_socket_identity(&path).unwrap(), original_identity);
    assert_eq!(
        std::fs::symlink_metadata(&path).unwrap().mode() & HOST_SOCKET_PERMISSION_MASK,
        REPLACEMENT_SOCKET_MODE
    );
    drop(replacement);
    drop(original);
}
