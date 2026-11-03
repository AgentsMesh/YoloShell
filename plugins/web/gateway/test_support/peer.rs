use super::{deadline, source::Source};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWrite};
use yolo_hub_client_access_dispatch::ScopedDispatcher;
use yolo_hub_client_transport::IncomingChannel;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture, TerminalSource};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{
    CLIENT_ACCESS_METHODS, Command, HUB_PROTOCOL_VERSION, Hello, Reply, Request, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

pub struct Backend {
    pub hub: HubId,
}

impl Dispatcher for Backend {
    fn hub_id(&self) -> HubId {
        self.hub
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            Ok(match request.fields().command {
                Command::Hello {} => Operation::Reply(Reply::Hello {
                    hello: Hello {
                        hub_id: self.hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: SUPPORTED_METHODS
                            .iter()
                            .chain(CLIENT_ACCESS_METHODS)
                            .copied()
                            .collect(),
                    },
                }),
                Command::List { .. } => Operation::Reply(Reply::Terminals {
                    terminals: Vec::new(),
                    next: None,
                }),
                Command::Attach { request } => {
                    Operation::Attached(Box::new(Source::new(request.target())))
                }
                _ => return Err(PublicError::Unsupported),
            })
        })
    }
}

pub async fn serve(incoming: IncomingChannel, hub: HubId) -> Result<()> {
    let dispatcher = ScopedDispatcher::new(Arc::new(Backend { hub }), incoming.admission)?;
    let mut channel = Framed::new(incoming.stream);
    while let Incoming::Message(call) = channel.receive::<Call>(deadline()).await? {
        match dispatcher.dispatch(call.request().clone()).await {
            Ok(Operation::Attached(source)) => {
                yolo_hub_terminal_wire::send_attachment(
                    &mut channel,
                    &call,
                    source.initial(),
                    deadline(),
                )
                .await?;
                let (mut reader, writer) = tokio::io::split(channel.into_stream()?);
                return tokio::select! {
                    result = output(Framed::new(writer), source) => result,
                    closed = reader.read_u8() => { let _ = closed; Ok(()) },
                };
            }
            operation => {
                let reply = match operation {
                    Ok(Operation::Reply(reply)) => reply,
                    Err(code) => Reply::Error { code },
                    _ => unreachable!(),
                };
                channel
                    .send(&Answer::new(&call, reply)?, deadline())
                    .await?;
            }
        }
    }
    Ok(())
}

async fn output<S: AsyncWrite + Unpin>(
    mut writer: Framed<S>,
    mut source: Box<dyn TerminalSource>,
) -> Result<()> {
    loop {
        let event = source.next_event().await?;
        yolo_hub_terminal_wire::send_event(&mut writer, &event, deadline()).await?;
    }
}
