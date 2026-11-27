use serde::{Deserialize, Serialize};
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method};

use crate::{ClientCapability, ClientFeatureRequirement};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientApiContract {
    pub protocol_version: u16,
    pub required_methods: Vec<Method>,
    #[serde(default)]
    pub optional_methods: Vec<Method>,
    #[serde(default)]
    pub features: Vec<ClientFeatureRequirement>,
}

impl ClientApiContract {
    pub fn new(
        protocol_version: u16,
        required_methods: impl Into<Vec<Method>>,
        optional_methods: impl Into<Vec<Method>>,
        features: impl Into<Vec<ClientFeatureRequirement>>,
    ) -> Self {
        Self {
            protocol_version,
            required_methods: required_methods.into(),
            optional_methods: optional_methods.into(),
            features: features.into(),
        }
    }

    pub fn permits_connection(&self, hello: &Hello) -> bool {
        self.protocol_version == hello.version
            && self
                .required_methods
                .iter()
                .all(|method| hello.methods.contains(method))
    }

    pub fn enabled_features(&self, hello: &Hello) -> Vec<ClientCapability> {
        if !self.permits_connection(hello) {
            return Vec::new();
        }
        self.features
            .iter()
            .filter(|feature| {
                feature
                    .methods
                    .iter()
                    .all(|method| hello.methods.contains(method))
            })
            .map(|feature| feature.capability)
            .collect()
    }
}
