use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use uuid::Uuid;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::{Command, Reply, Request};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, PublicError,
    TERMINAL_PROTOCOL_SCHEMA_VERSION, SecretCapability, TerminalSize,
};

const CREATION_DIRECTORY: &str = "/tmp";
pub const SINGLE_CALL: usize = 1;
pub const NO_COMPLETIONS: usize = 0;

pub struct DelayedDispatcher {
    pub hub: HubId,
    pub delay: Duration,
    pub result: Result<Reply, PublicError>,
    pub started: AtomicUsize,
    pub completed: AtomicUsize,
    pub dropped: AtomicUsize,
}

impl DelayedDispatcher {
    pub fn new(delay: Duration) -> Self {
        let hub = HubId::new();
        Self {
            hub,
            delay,
            result: Ok(Reply::Created {
                creation: CreateShellResult {
                    address: TerminalAddress {
                        hub_id: hub,
                        terminal_id: TerminalId::new(),
                    },
                    creation_capability: SecretCapability::generate(),
                },
            }),
            started: AtomicUsize::new(NO_COMPLETIONS),
            completed: AtomicUsize::new(NO_COMPLETIONS),
            dropped: AtomicUsize::new(NO_COMPLETIONS),
        }
    }

    pub fn creation(&self) -> Request {
        Request::new(
            self.hub,
            Command::Create {
                request: CreateShellRequest::new(CreateShellFields {
                    schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
                    request_id: Uuid::new_v4(),
                    creation_token: Uuid::new_v4(),
                    name: None,
                    cwd: CREATION_DIRECTORY.into(),
                    size: TerminalSize::default(),
                })
                .unwrap(),
            },
        )
        .unwrap()
    }

    pub fn assert_counts(&self, completed: usize) {
        assert_eq!(self.started.load(Ordering::SeqCst), SINGLE_CALL);
        assert_eq!(self.completed.load(Ordering::SeqCst), completed);
        assert_eq!(self.dropped.load(Ordering::SeqCst), SINGLE_CALL);
    }
}

impl Dispatcher for DelayedDispatcher {
    fn hub_id(&self) -> HubId {
        self.hub
    }
    fn dispatch(&self, _: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            self.started.fetch_add(SINGLE_CALL, Ordering::SeqCst);
            let _guard = DropGuard(&self.dropped);
            tokio::time::sleep(self.delay).await;
            self.completed.fetch_add(SINGLE_CALL, Ordering::SeqCst);
            self.result.clone().map(Operation::Reply)
        })
    }
}

struct DropGuard<'a>(&'a AtomicUsize);
impl Drop for DropGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(SINGLE_CALL, Ordering::SeqCst);
    }
}
