use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Hello, Reply, Request, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary, TerminalSummaryFields};

pub struct Backend {
    pub hub: HubId,
    pub allowed: HubId,
    pub hidden: HubId,
    pub calls: AtomicUsize,
}
impl Dispatcher for Backend {
    fn hub_id(&self) -> HubId {
        self.hub
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let reply = match request.fields().command {
                Command::Hello {} => Reply::Hello {
                    hello: Hello {
                        hub_id: self.hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: SUPPORTED_METHODS.to_vec(),
                    },
                },
                Command::List { .. } => {
                    let mut terminals = [self.allowed, self.hidden]
                        .map(|hub_id| {
                            TerminalSummary::new(TerminalSummaryFields {
        presentation: Default::default(),
                                address: yolo_hub_identity::TerminalAddress {
                                    hub_id,
                                    terminal_id: yolo_hub_identity::TerminalId::new(),
                                },
                                name: SESSION_NAME.into(),
                                cwd: SESSION_DIRECTORY.into(),
                                archived: false,
                                recovery: None,
                                runtime: None,
                                agent: None,
                                workspace: None,
                            })
                            .unwrap()
                        })
                        .to_vec();
                    terminals.sort_by_key(|session| session.fields().address);
                    Reply::Terminals {
                        terminals,
                        next: None,
                    }
                }
                _ => return Err(PublicError::Unsupported),
            };
            Ok(Operation::Reply(reply))
        })
    }
}
