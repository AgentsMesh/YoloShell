use super::super::*;
use super::{Fixture, bind_private_socket};

const COLLIDING_QUARANTINE_SOCKET: &str = ".broker-quarantine-collision.sock";
const RETRY_QUARANTINE_SOCKET: &str = ".broker-quarantine-retry.sock";
const QUARANTINE_NAME_SOURCE_EXHAUSTED: &str = "quarantine name source exhausted";

#[test]
fn quarantine_retries_a_colliding_destination_before_removal() {
    let fixture = Fixture::new();
    let listener = fixture.bind();
    let expected = fixture.identity();
    let collision_path = fixture.path.join(COLLIDING_QUARANTINE_SOCKET);
    let occupied = bind_private_socket(&collision_path);
    let retry_path = fixture.path.join(RETRY_QUARANTINE_SOCKET);
    let mut names = [COLLIDING_QUARANTINE_SOCKET, RETRY_QUARANTINE_SOCKET].into_iter();
    let mut next_name = || {
        let name = names.next().expect(QUARANTINE_NAME_SOURCE_EXHAUSTED);
        Ok(CString::new(name).unwrap())
    };

    fixture
        .directory
        .remove_expected_using(&fixture.name, expected, || Ok(()), &mut next_name)
        .unwrap();

    assert!(!fixture.socket_path().exists());
    assert!(collision_path.exists());
    assert!(!retry_path.exists());
    drop(occupied);
    drop(listener);
}

#[test]
fn quarantine_collision_exhaustion_preserves_the_expected_socket() {
    let fixture = Fixture::new();
    let listener = fixture.bind();
    let expected = fixture.identity();
    let collision_path = fixture.path.join(COLLIDING_QUARANTINE_SOCKET);
    let occupied = bind_private_socket(&collision_path);
    let mut next_name = || Ok(CString::new(COLLIDING_QUARANTINE_SOCKET).unwrap());

    let error = fixture
        .directory
        .remove_expected_using(&fixture.name, expected, || Ok(()), &mut next_name)
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(BROKER_SOCKET_QUARANTINE_EXHAUSTED)
    );
    assert_eq!(fixture.identity(), expected);
    assert!(collision_path.exists());
    drop(occupied);
    drop(listener);
}
