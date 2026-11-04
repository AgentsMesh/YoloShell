use super::super::*;
use super::{Fixture, bind_private_socket};
use std::cell::Cell;

const COLLIDING_STAGING_SOCKET: &str = "colliding-staging.sock";
const INVALID_STAGING_SOCKET: &str = "nested/staging.sock";
const RETRY_STAGING_SOCKET: &str = "retry-staging.sock";
const STAGING_NAME_SOURCE_EXHAUSTED: &str = "staging name source exhausted";
const NAME_SOURCE_FAILURE: &str = "injected staging name failure";

#[tokio::test]
async fn deterministic_staging_names_retry_collisions_and_bound_attempts() {
    let fixture = Fixture::new();
    let occupied_path = fixture.state_path.join(COLLIDING_STAGING_SOCKET);
    let occupied = bind_private_socket(&occupied_path);
    let before_called = Cell::new(false);
    let after_called = Cell::new(false);
    let mut names = [COLLIDING_STAGING_SOCKET, RETRY_STAGING_SOCKET].into_iter();

    let bound = bind_private_using(
        &fixture.state,
        Fixture::target_name(),
        || {
            before_called.set(true);
            Ok(())
        },
        |_, _| after_called.set(true),
        || {
            let name = names.next().expect(STAGING_NAME_SOURCE_EXHAUSTED);
            Ok(CString::new(name).unwrap())
        },
    )
    .unwrap();

    assert!(before_called.get());
    assert!(after_called.get());
    drop(bound);

    let error = bind_private_using(
        &fixture.state,
        Fixture::target_name(),
        || Ok(()),
        |_, _| {},
        || Ok(CString::new(COLLIDING_STAGING_SOCKET).unwrap()),
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains(BROKER_SOCKET_STAGING_EXHAUSTED));
    assert!(occupied_path.exists());
    assert!(!fixture.target_path().exists());
    drop(occupied);
}

#[tokio::test]
async fn staging_name_source_failure_is_propagated_before_binding() {
    let fixture = Fixture::new();

    let error = bind_private_using(
        &fixture.state,
        Fixture::target_name(),
        || Ok(()),
        |_, _| {},
        || anyhow::bail!(NAME_SOURCE_FAILURE),
    )
    .err()
    .unwrap();

    assert!(error.to_string().contains(NAME_SOURCE_FAILURE));
    assert!(!fixture.target_path().exists());
}

#[tokio::test]
async fn non_collision_bind_failure_is_contextualized_without_publication() {
    let fixture = Fixture::new();

    let error = bind_private_using(
        &fixture.state,
        Fixture::target_name(),
        || Ok(()),
        |_, _| {},
        || Ok(CString::new(INVALID_STAGING_SOCKET).unwrap()),
    )
    .err()
    .unwrap();

    assert!(
        format!("{error:#}").contains(BROKER_SOCKET_STAGING_BIND_CONTEXT),
        "{error:#}"
    );
    assert!(!fixture.target_path().exists());
}
