use super::{OPERATION_TIMEOUT, connection, fixture};
use std::sync::Arc;
use tokio::{io::AsyncWriteExt, time::Instant};
use yolo_hub_jsonrpc_protocol::{
    Answer, Call, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::Command;
use yolo_local_unix::LocalUnixStream;

fn deadline() -> Instant {
    Instant::now() + OPERATION_TIMEOUT
}

#[tokio::test]
async fn malformed_calls_have_standard_errors_and_do_not_poison_an_authenticated_connection() {
    let (_root, state, backend) = fixture();
    let hello = Call::new(state.hub_id(), Command::Hello {}).unwrap();
    let (mut left, right) = LocalUnixStream::pair().unwrap();
    let serving = connection::serve(right, Arc::new(backend), Arc::new(state));
    let client = async {
        let frames = [
            (b"{broken}\n".as_slice(), PARSE_ERROR),
            (
                b"{\"jsonrpc\":\"2.0\",\"id\":1,\"id\":2,\"method\":\"hello\",\"params\":{}}\n",
                PARSE_ERROR,
            ),
            (b"[]\n", INVALID_REQUEST),
            (
                b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"unknown\",\"params\":{}}\n",
                METHOD_NOT_FOUND,
            ),
            (
                b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"create\",\"params\":{}}\n",
                INVALID_PARAMS,
            ),
        ];
        for (bytes, _) in &frames {
            left.write_all(bytes).await.unwrap();
        }
        let mut channel = Framed::new(left);
        for (_, code) in frames {
            let Incoming::Message(answer) = channel.receive::<Answer>(deadline()).await.unwrap()
            else {
                panic!()
            };
            assert_eq!(answer.error.unwrap().code, code);
        }
        channel.send(&hello, deadline()).await.unwrap();
        let Incoming::Message(answer) = channel.receive::<Answer>(deadline()).await.unwrap() else {
            panic!()
        };
        assert!(answer.reply(&hello).is_ok());
    };
    let (result, ()) = tokio::join!(serving, client);
    result.unwrap();
}

#[tokio::test]
async fn notification_mutations_are_ignored_without_a_reply_or_contacting_the_host() {
    let (_root, state, backend) = fixture();
    let hello = Call::new(state.hub_id(), Command::Hello {}).unwrap();
    let (mut left, right) = LocalUnixStream::pair().unwrap();
    let serving = connection::serve(right, Arc::new(backend), Arc::new(state));
    let client = async {
        left.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"create\",\"params\":{}}\n")
            .await
            .unwrap();
        let mut channel = Framed::new(left);
        channel.send(&hello, deadline()).await.unwrap();
        let Incoming::Message(answer) = channel.receive::<Answer>(deadline()).await.unwrap() else {
            panic!()
        };
        assert!(answer.reply(&hello).is_ok());
    };
    let (result, ()) = tokio::join!(serving, client);
    result.unwrap();
}
