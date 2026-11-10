use std::{future::Future, pin::Pin, sync::Arc};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Reply, Request};
use yolo_hub_terminal_protocol::{AttachResult, PublicError, TerminalEvent};

pub type OperationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Operation, PublicError>> + Send + 'a>>;
pub type EventFuture<'a> =
    Pin<Box<dyn Future<Output = Result<TerminalEvent, PublicError>> + Send + 'a>>;
pub enum Operation {
    Reply(Reply),
    Attached(Box<dyn TerminalSource>),
}
pub trait Dispatcher: Send + Sync {
    fn hub_id(&self) -> HubId;
    fn dispatch(&self, request: Request) -> OperationFuture<'_>;
}
/// Decorates a local backend for authenticated peer-origin calls, which
/// bypass the public client-access chain but still need origin-side services.
pub trait OriginDecorator<B>: Send + Sync {
    fn decorate(&self, backend: B) -> Arc<dyn Dispatcher>;
}
/// Dropping a source detaches an observer; it must never terminate the source PTY.
pub trait TerminalSource: Send {
    fn initial(&self) -> &AttachResult;
    fn next_event(&mut self) -> EventFuture<'_>;
}
mod build;
pub use build::{BuildFuture, BuildProvider};
