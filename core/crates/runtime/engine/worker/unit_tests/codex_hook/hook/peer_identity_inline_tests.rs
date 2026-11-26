use super::verify;

#[cfg(feature = "yolo-unit-tests")]
#[tokio::test(flavor = "current_thread")]
async fn verification_captures_the_exact_peer_generation() {
    let (server, _client) = crate::local_unix::LocalUnixStream::pair().unwrap();
    let expected = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();

    assert_eq!(verify(&server).unwrap(), expected);
}
