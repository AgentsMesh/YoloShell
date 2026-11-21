//! Machine-readable index of the current public Hub contract, not a permission grant.
use serde::Serialize;
use yolo_hub_routing::Rights;
use yolo_hub_rpc_protocol::{ALL_METHODS, Method};

pub const DESCRIPTOR_SCHEMA_VERSION: u16 = 1;
pub const PROTOCOL_NAME: &str = "yolo.hub";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AccessRequirement {
    LocalManagement,
    Granted { rights: Rights },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplyChannel {
    Control,
    TerminalStream,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MethodDescriptor {
    pub method: Method,
    pub access: AccessRequirement,
    pub reply_channel: ReplyChannel,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProtocolDescriptor {
    pub schema_version: u16,
    pub protocol: &'static str,
    pub version: u16,
    pub methods: Vec<MethodDescriptor>,
}

impl ProtocolDescriptor {
    /// Describes this image's implementations; a live Hub may advertise a subset.
    pub fn current() -> Self {
        Self::from_methods(
            yolo_hub_rpc_protocol::HUB_PROTOCOL_VERSION,
            compiled_methods(),
        )
    }

    fn from_methods(version: u16, methods: &[Method]) -> Self {
        let mut entries = Vec::new();
        for &method in methods {
            entries.push(MethodDescriptor {
                method,
                access: match method.required_rights() {
                    Some(rights) => AccessRequirement::Granted { rights },
                    None => AccessRequirement::LocalManagement,
                },
                reply_channel: if method == Method::Attach {
                    ReplyChannel::TerminalStream
                } else {
                    ReplyChannel::Control
                },
            });
        }
        Self {
            schema_version: DESCRIPTOR_SCHEMA_VERSION,
            protocol: PROTOCOL_NAME,
            version,
            methods: entries,
        }
    }
}

pub fn compiled_methods() -> &'static [Method] {
    ALL_METHODS
}
