mod dispatcher;

pub use dispatcher::ProviderDispatcher;

/// Origin-side methods implemented by this decorator.
pub const ORIGIN_METHODS: &[yolo_hub_rpc_protocol::Method] =
    &[yolo_hub_rpc_protocol::Method::ListProviders];

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
