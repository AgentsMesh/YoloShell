use std::collections::BTreeMap;

use yolo_builtin_client_contract::{
    BuiltinClientDescriptor, CapabilityNegotiation, ProcessEntrypoint, negotiate,
};
use yolo_hub_rpc_protocol::Hello;
use yolo_plugin_api::Platform;

use crate::ClientHostError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientSessionDeclaration {
    pub client_id: String,
    pub entrypoint: ProcessEntrypoint,
    pub capabilities: CapabilityNegotiation,
}

#[derive(Clone, Debug, Default)]
pub struct BuiltinClientHost {
    clients: BTreeMap<String, BuiltinClientDescriptor>,
}

impl BuiltinClientHost {
    pub fn builtin(
        descriptors: impl IntoIterator<Item = BuiltinClientDescriptor>,
    ) -> Result<Self, ClientHostError> {
        let mut host = Self::default();
        for descriptor in descriptors {
            host.register(descriptor)?;
        }
        Ok(host)
    }

    pub fn register(&mut self, descriptor: BuiltinClientDescriptor) -> Result<(), ClientHostError> {
        if self.clients.contains_key(descriptor.id()) {
            return Err(ClientHostError::DuplicateClient);
        }
        self.clients.insert(descriptor.id().to_owned(), descriptor);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&BuiltinClientDescriptor> {
        self.clients.get(id)
    }

    pub fn negotiate(
        &self,
        id: &str,
        hello: &Hello,
    ) -> Result<CapabilityNegotiation, ClientHostError> {
        let descriptor = self.clients.get(id).ok_or(ClientHostError::UnknownClient)?;
        Ok(negotiate(descriptor.api(), hello))
    }

    pub fn start(
        &self,
        id: &str,
        hello: &Hello,
    ) -> Result<ClientSessionDeclaration, ClientHostError> {
        let descriptor = self.clients.get(id).ok_or(ClientHostError::UnknownClient)?;
        self.start_on(id, descriptor.platform(), hello)
    }

    pub fn start_on(
        &self,
        id: &str,
        platform: Platform,
        hello: &Hello,
    ) -> Result<ClientSessionDeclaration, ClientHostError> {
        let descriptor = self.clients.get(id).ok_or(ClientHostError::UnknownClient)?;
        if !descriptor.platform().runs_on(platform) {
            return Err(ClientHostError::UnsupportedPlatform(platform));
        }
        let capabilities = self.negotiate(id, hello)?;
        if !capabilities.can_start() {
            return Err(ClientHostError::StartupUnsupported);
        }
        Ok(ClientSessionDeclaration {
            client_id: id.to_owned(),
            entrypoint: descriptor.entrypoint().clone(),
            capabilities,
        })
    }
}
