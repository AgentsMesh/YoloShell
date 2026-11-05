use super::support::{ProxyReply, RUNTIME_HOME};
use super::support_channel_probe::ChannelProbe;
use crate::host::HelloResult;
use crate::model::HostStatus;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method};
use crate::remote::{BoxedRemoteChannel, ChannelIdentity};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf, duplex};
use tokio::task::AbortHandle;

pub const FAKE_CHANNEL_CAPACITY: usize = 65_536;

#[derive(Clone, Copy)]
pub enum OpenStep {
    Channel,
    BlockedOpen,
    GatedHandshakeWrite,
    StalledChannel,
    WriteStall,
    ShutdownStall,
    TransportFailure,
    Rejected,
    Mismatch,
    PurposeMismatch,
    ImmediateEof,
    EofAfterProbe,
}

pub fn open_channel(
    step: OpenStep,
    identity: ChannelIdentity,
    probe: Arc<ChannelProbe>,
) -> BoxedRemoteChannel {
    let (client, server) = duplex(FAKE_CHANNEL_CAPACITY);
    let business_request_received = Arc::new(AtomicBool::new(false));
    let task = tokio::spawn(serve(
        server,
        identity,
        step,
        Arc::clone(&business_request_received),
    ));
    Box::new(FaultChannel {
        stream: client,
        step,
        probe,
        business_request_received,
        abort: task.abort_handle(),
    })
}

async fn serve(stream: DuplexStream, identity: ChannelIdentity, step: OpenStep,
    business_request_received: Arc<AtomicBool>) {
    if matches!(step, OpenStep::ImmediateEof) { return; }
    if matches!(step, OpenStep::StalledChannel) { std::future::pending::<()>().await; }
    let stop = Arc::new(tokio::sync::Notify::new());
    let service = Arc::new(RuntimeService { identity, step, business_request_received,
        hello_received: AtomicBool::new(false), completed: AtomicBool::new(false), stop: stop.clone() });
    tokio::select! {
        _ = stop.notified() => {},
        _ = yolo_capnp_rpc::serve(stream, yolo_capnp_rpc::Protocol::Endpoint, service) => {},
    }
}
struct RuntimeService {
    identity: ChannelIdentity, step: OpenStep, business_request_received: Arc<AtomicBool>,
    hello_received: AtomicBool, completed: AtomicBool, stop: Arc<tokio::sync::Notify>,
}
impl yolo_capnp_rpc::Service for RuntimeService {
    fn call(&self, _: yolo_capnp_rpc::Method, value: yolo_capnp_rpc::Value) -> yolo_capnp_rpc::Future<'_, yolo_capnp_rpc::Reply> {
        Box::pin(async move {
            if self.completed.load(Ordering::Acquire) { return Err(yolo_capnp_rpc::Error::Closed); }
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            let is_hello = request.method == method::HELLO;
            let is_probe_status = self.hello_received.load(Ordering::SeqCst) && request.method == method::HOST_STATUS;
            let response = if is_hello {
                self.hello_received.store(true, Ordering::SeqCst);
                ResponseEnvelope::success(&request, &HelloResult {
                    server_name: crate::protocol::HOST_SERVER_NAME.into(),
                    protocol_version: crate::model::PROTOCOL_VERSION, capabilities: required_capabilities(),
                })
            } else if is_probe_status {
                self.hello_received.store(false, Ordering::SeqCst);
                ResponseEnvelope::success(&request, &probe_status())
            } else {
                self.business_request_received.store(true, Ordering::Release);
                ResponseEnvelope::success(&request, &ProxyReply { connection_id: self.identity.connection_id, channel_id: self.identity.channel_id })
            }.map_err(|_| yolo_capnp_rpc::Error::Invalid)?;
            let stop = self.stop.clone();
            let close = matches!(self.step, OpenStep::EofAfterProbe) && is_probe_status;
            if close { self.completed.store(true, Ordering::Release); }
            Ok(yolo_capnp_rpc::Reply { value: yolo_capnp_codec::to_value(&response)?, events: None,
                on_observed: close.then(|| Box::new(move || stop.notify_one()) as Box<dyn yolo_capnp_rpc::Confirmation>) })
        })
    }
}

fn required_capabilities() -> Vec<String> {
    crate::remote::REQUIRED_RUNTIME_CAPABILITIES
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn probe_status() -> HostStatus {
    HostStatus {
        protocol_version: crate::model::PROTOCOL_VERSION,
        boot_id: String::new(),
        host_pid: u32::default(),
        controller_generation: u64::default(),
        home: RUNTIME_HOME.into(),
        capabilities: required_capabilities(),
    }
}

struct FaultChannel {
    stream: DuplexStream,
    step: OpenStep,
    probe: Arc<ChannelProbe>,
    business_request_received: Arc<AtomicBool>,
    abort: AbortHandle,
}

impl Drop for FaultChannel {
    fn drop(&mut self) {
        self.probe.dropped();
        self.abort.abort();
    }
}

impl AsyncRead for FaultChannel {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buffer)
    }
}

impl AsyncWrite for FaultChannel {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let gated = matches!(self.step, OpenStep::GatedHandshakeWrite)
            && !self.probe.writes_allowed.load(Ordering::Acquire);
        let initial_stall = matches!(self.step, OpenStep::StalledChannel);
        let stalled = matches!(self.step, OpenStep::WriteStall)
            && self.business_request_received.load(Ordering::Acquire);
        if initial_stall || gated || stalled {
            self.probe.pending_write(cx);
            return Poll::Pending;
        }
        Pin::new(&mut self.stream).poll_write(cx, buffer)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if matches!(self.step, OpenStep::ShutdownStall) {
            self.probe.pending_shutdown();
            Poll::Pending
        } else {
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }
}
