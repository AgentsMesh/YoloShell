use std::sync::Mutex;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

pub struct Backend {
    pub hub: HubId,
    pub methods: Vec<Method>,
    calls: Mutex<Vec<Request>>,
}

impl Backend {
    pub fn new(hub: HubId) -> Self {
        Self {
            hub,
            methods: vec![Method::Hello, Method::List],
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<Request> {
        self.calls.lock().unwrap().clone()
    }
}

impl Dispatcher for Backend {
    fn hub_id(&self) -> HubId {
        self.hub
    }

    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        self.calls.lock().unwrap().push(request.clone());
        Box::pin(async move {
            let reply = match request.fields().command {
                Command::Hello {} => Reply::Hello {
                    hello: Hello {
                        hub_id: self.hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: self.methods.clone(),
                    },
                },
                Command::List { .. } => Reply::Terminals {
                    terminals: Vec::new(),
                    next: None,
                },
                _ => return Err(PublicError::Unsupported),
            };
            Ok(Operation::Reply(reply))
        })
    }
}
