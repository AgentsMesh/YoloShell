use std::{future::Future, pin::Pin};
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_terminal_protocol::PublicError;

pub type BuildFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ServiceBuild, PublicError>> + Send + 'a>>;

pub trait BuildProvider: Send + Sync {
    fn build(&self) -> BuildFuture<'_>;
}
