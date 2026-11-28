use serde::Deserialize;
use yolo_hub_rpc_protocol::Hello;
use yolo_plugin_api::{ClientApiContract, Platform};

use crate::{Executable, ProcessEntrypoint};

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClientDescriptorError {
    #[error("built-in client id is empty")]
    Identifier,
    #[error("built-in client API contract is invalid")]
    Protocol,
    #[error("built-in client entrypoint platform does not match its platform")]
    Platform,
    #[error("built-in client declares a method outside its API contract")]
    UndeclaredCapability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltinClientDescriptor {
    id: String,
    platform: Platform,
    entrypoint: ProcessEntrypoint,
    api: ClientApiContract,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DescriptorFields {
    id: String,
    platform: Platform,
    entrypoint: ProcessEntrypoint,
    api: ClientApiContract,
}

impl TryFrom<DescriptorFields> for BuiltinClientDescriptor {
    type Error = ClientDescriptorError;
    fn try_from(fields: DescriptorFields) -> Result<Self, Self::Error> {
        Self::new(fields.id, fields.platform, fields.entrypoint, fields.api)
    }
}

impl BuiltinClientDescriptor {
    pub fn new(
        id: impl Into<String>,
        platform: Platform,
        entrypoint: ProcessEntrypoint,
        api: ClientApiContract,
    ) -> Result<Self, ClientDescriptorError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ClientDescriptorError::Identifier);
        }
        if entrypoint.platform != platform {
            return Err(ClientDescriptorError::Platform);
        }
        if api.protocol_version == u16::MIN
            || api.required_methods.is_empty()
            || api
                .required_methods
                .iter()
                .enumerate()
                .any(|(index, method)| api.required_methods[..index].contains(method))
            || api.optional_methods.iter().any(|method| {
                api.required_methods.contains(method)
                    || api
                        .optional_methods
                        .iter()
                        .filter(|item| *item == method)
                        .count()
                        > 1
            })
            || api
                .features
                .iter()
                .flat_map(|feature| &feature.methods)
                .any(|method| {
                    !api.required_methods.contains(method) && !api.optional_methods.contains(method)
                })
        {
            return Err(ClientDescriptorError::Protocol);
        }
        Ok(Self {
            id,
            platform,
            entrypoint,
            api,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn entrypoint(&self) -> &ProcessEntrypoint {
        &self.entrypoint
    }

    pub fn api(&self) -> &ClientApiContract {
        &self.api
    }

    pub fn permits_connection(&self, hello: &Hello) -> bool {
        self.api.permits_connection(hello)
    }
}

impl BuiltinClientDescriptor {
    pub fn yolo_image(id: impl Into<String>, platform: Platform, api: ClientApiContract) -> Self {
        Self::new(
            id,
            platform,
            ProcessEntrypoint {
                platform,
                executable: Executable::YoloImage,
                arguments: Vec::new(),
            },
            api,
        )
        .expect("bundled TUI descriptor")
    }
}
