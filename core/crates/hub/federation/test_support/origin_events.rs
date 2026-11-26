use super::gate::TEST_TIMEOUT;
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};
use yolo_hub_dispatch::{EventFuture, TerminalSource};
use yolo_hub_peer_protocol::{Call, FIRST_STREAM_SEQUENCE, Signed};
use yolo_hub_peer_rpc::{RpcError, Updates};
use yolo_hub_terminal_protocol::{AttachResult, OutputCursor, PublicError, TerminalEvent};

const SOURCE_QUEUE_CAPACITY: usize = 1;
pub const TERMINAL_OUTPUT: &[u8] = "agent 输出\r\n".as_bytes();
pub const SUCCESS_EXIT_CODE: i32 = 0;

pub struct Source {
    initial: AttachResult,
    receiver: mpsc::Receiver<Result<TerminalEvent, PublicError>>,
    dropped: Option<oneshot::Sender<()>>,
}
pub struct Feed {
    pub events: mpsc::Sender<Result<TerminalEvent, PublicError>>,
    pub dropped: oneshot::Receiver<()>,
}

impl Source {
    pub fn new(initial: AttachResult) -> (Box<Self>, Feed) {
        let (events, receiver) = mpsc::channel(SOURCE_QUEUE_CAPACITY);
        let (dropped, released) = oneshot::channel();
        (
            Box::new(Self {
                initial,
                receiver,
                dropped: Some(dropped),
            }),
            Feed {
                events,
                dropped: released,
            },
        )
    }
}
impl TerminalSource for Source {
    fn initial(&self) -> &AttachResult {
        &self.initial
    }
    fn next_event(&mut self) -> EventFuture<'_> {
        Box::pin(async {
            self.receiver
                .recv()
                .await
                .unwrap_or(Err(PublicError::Unreachable))
        })
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        let _ = self.dropped.take().unwrap().send(());
    }
}

/// Check signed stream identity, sequence and terminal cursor even for intervening heartbeats.
pub async fn content(
    updates: &mut dyn Updates,
    call: &Signed<Call>,
    sequence: &mut u64,
    cursor: &mut OutputCursor,
) -> Result<TerminalEvent, RpcError> {
    timeout(TEST_TIMEOUT, async {
        loop {
            let update = updates.next().await?;
            update
                .validate_for(call, *sequence, crate::node::now_ms().unwrap())
                .unwrap();
            *sequence += FIRST_STREAM_SEQUENCE;
            let event = update.payload.event;
            *cursor = event.advance(*cursor).unwrap();
            if !matches!(event, TerminalEvent::Heartbeat { .. }) {
                return Ok(event);
            }
        }
    })
    .await
    .unwrap()
}
