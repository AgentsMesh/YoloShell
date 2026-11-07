use super::*;
use crate::protocol::event;
use tokio::net::UnixStream;

#[tokio::test]
async fn fragmented_event_rejects_an_empty_transport_fragment() {
    let event = EventEnvelope::new(event::SESSION_EXITED, None, None, &()).unwrap();
    let fragment_bytes = 0;
    {
        let (stream, _client) = UnixStream::pair().unwrap();
        let (stream, transport) = Transport::new(stream);
        let peer = Peer::new(stream);
        let (fragment_sent, _fragment_received) = oneshot::channel();
        let (attachment_ready, ready) = oneshot::channel();
        attachment_ready.send(()).unwrap();
        let fragmented = FragmentedEvent::new(
            event.clone(),
            ready,
            fragment_bytes,
            fragment_sent,
            std::sync::Arc::new(Notify::new()),
        );
        let error = fragmented.send(&peer, &transport).await.unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::Io(error) if error.kind() == std::io::ErrorKind::InvalidInput
        ));
    }
}
