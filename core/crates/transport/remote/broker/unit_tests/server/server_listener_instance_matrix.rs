use super::*;
use std::io;

const ACCEPT_FAILURE: &str = "injected listener acceptance failure";
const INITIAL_ACCEPTED_COUNT: usize = 0;
const EXPECTED_ACCEPTED_COUNT: usize = 1;

#[derive(Clone, Copy)]
enum Action {
    Accepted,
    Failed,
}

#[tokio::test(start_paused = true)]
async fn one_listener_callback_instance_covers_acceptance_and_backoff() {
    let mut accepted_count = INITIAL_ACCEPTED_COUNT;
    let mut backoff = crate::ingress::AcceptBackoff::new(
        ACCEPT_FAILURE_INITIAL_BACKOFF,
        ACCEPT_FAILURE_MAX_BACKOFF,
    );
    for action in [Action::Failed, Action::Accepted] {
        let mut peer = None;
        let result = match action {
            Action::Accepted => {
                let (stream, retained) = UnixStream::pair().unwrap();
                peer = Some(retained);
                Ok(stream)
            }
            Action::Failed => Err(io::Error::other(ACCEPT_FAILURE)),
        };
        let mut on_stream = |_stream| accepted_count += EXPECTED_ACCEPTED_COUNT;
        handle_listener_result(result, &mut backoff, RUNTIME_LISTENER_LABEL, &mut on_stream).await;
        drop(peer);
    }
    assert_eq!(accepted_count, EXPECTED_ACCEPTED_COUNT);
}
