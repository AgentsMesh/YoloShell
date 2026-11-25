use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;

const MAX_ABORT_BYTES: usize = 4096;
const EXCESS_BYTE: usize = 1;
const WORD_BYTES: usize = std::mem::size_of::<u64>();
const ABORT_NESTING_LIMIT: i32 = 8;
const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const SHUTDOWN_DEADLINE: &str = "RPC shutdown tail did not reach EOF";

pub(super) async fn assert_closed(stream: UnixStream) {
    let deadline = std::time::Instant::now() + SHUTDOWN_TIMEOUT;
    let mut bytes = Vec::new();
    let mut reader = stream.take((MAX_ABORT_BYTES + EXCESS_BYTE) as u64);
    {
        let reading = reader.read_to_end(&mut bytes);
        tokio::pin!(reading);
        loop {
            tokio::select! {
                biased;
                result = &mut reading => { result.unwrap(); break; },
                _ = tokio::task::yield_now() => {}
            }
            assert!(std::time::Instant::now() < deadline, "{SHUTDOWN_DEADLINE}");
        }
    }
    assert!(bytes.len() <= MAX_ABORT_BYTES);
    // A connection cancelled before RPC activation may close without any message.
    if bytes.is_empty() {
        return;
    }
    let mut remaining = bytes.as_slice();
    let message = capnp::serialize::read_message_from_flat_slice(
        &mut remaining,
        capnp::message::ReaderOptions {
            traversal_limit_in_words: Some(MAX_ABORT_BYTES / WORD_BYTES),
            nesting_limit: ABORT_NESTING_LIMIT,
        },
    )
    .unwrap();
    assert!(remaining.is_empty(), "unexpected data after RPC shutdown");
    let root = message
        .get_root::<capnp_rpc::rpc_capnp::message::Reader<'_>>()
        .unwrap();
    let capnp_rpc::rpc_capnp::message::Abort(exception) = root.which().unwrap() else {
        panic!("silent RPC peer received a message other than shutdown");
    };
    assert_eq!(
        exception.unwrap().get_type().unwrap(),
        capnp_rpc::rpc_capnp::exception::Type::Disconnected
    );
}
