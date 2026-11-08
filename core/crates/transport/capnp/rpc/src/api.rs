use crate::{Value, constants::*};
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};

pub type Future<'a, T> = Pin<Box<dyn std::future::Future<Output = Result<T, Error>> + Send + 'a>>;
pub trait Duplex: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Duplex for T {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Protocol {
    Endpoint,
    Peer,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    Call,
    Discover,
    Execute,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Closed,
    Capacity,
    Invalid,
    Deadline,
    Runtime,
    OutcomeUnknown,
    Remote(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Closed => CLOSED,
            Self::Capacity => OVERLOADED,
            Self::Invalid => INVALID,
            Self::Deadline => DEADLINE,
            Self::Runtime => RUNTIME,
            Self::OutcomeUnknown => UNKNOWN,
            Self::Remote(message) => message,
        })
    }
}
impl std::error::Error for Error {}
impl From<capnp::Error> for Error {
    fn from(error: capnp::Error) -> Self {
        match error.kind {
            capnp::ErrorKind::Disconnected => Self::Closed,
            // In particular, remote Overloaded never means local admission Capacity.
            _ => Self::Remote(error.to_string()),
        }
    }
}
impl From<yolo_capnp_codec::ValueError> for Error {
    fn from(_: yolo_capnp_codec::ValueError) -> Self {
        Self::Invalid
    }
}

pub trait Service: Send + Sync + 'static {
    fn call(&self, method: Method, value: Value) -> Future<'_, Reply>;
}
pub trait EventSource: Send + 'static {
    fn next(&mut self) -> Future<'_, Value>;
}
pub trait Confirmation: Send + 'static {
    fn confirm(self: Box<Self>) -> Future<'static, ()>;
}
impl<F: FnOnce() + Send + 'static> Confirmation for F {
    fn confirm(self: Box<Self>) -> Future<'static, ()> {
        Box::pin(async move {
            (*self)();
            Ok(())
        })
    }
}
pub struct Reply {
    pub value: Value,
    pub events: Option<Box<dyn EventSource>>,
    /// A successful confirmation reply means this action completed, including upstream proxy confirmation.
    pub on_observed: Option<Box<dyn Confirmation>>,
}
impl Reply {
    pub fn value(value: Value) -> Self {
        Self {
            value,
            events: None,
            on_observed: None,
        }
    }
}
pub(crate) type SharedService = std::sync::Weak<dyn Service>;
pub(crate) fn rpc_error(error: Error) -> capnp::Error {
    match error {
        Error::Closed => capnp::Error::disconnected(error.to_string()),
        _ => capnp::Error::failed(error.to_string()),
    }
}
