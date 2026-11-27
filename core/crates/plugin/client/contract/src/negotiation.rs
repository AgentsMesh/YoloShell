use yolo_hub_rpc_protocol::{Hello, Method};
use yolo_plugin_api::ClientApiContract;

use crate::{CapabilityNegotiation, DisabledFeature};

/// Compare a built-in client API contract with a live Hub advertisement.
pub fn negotiate(api: &ClientApiContract, hello: &Hello) -> CapabilityNegotiation {
    let protocol_matches = api.protocol_version == hello.version;
    let startup_missing = missing(&api.required_methods, &hello.methods);
    let mut enabled_features = Vec::new();
    let mut disabled_features = Vec::new();
    for feature in &api.features {
        let missing_methods = missing(&feature.methods, &hello.methods);
        if protocol_matches && missing_methods.is_empty() {
            enabled_features.push(feature.capability);
        } else {
            disabled_features.push(DisabledFeature {
                capability: feature.capability,
                missing_methods,
            });
        }
    }
    CapabilityNegotiation {
        protocol_matches,
        startup_missing,
        enabled_features,
        disabled_features,
    }
}

fn missing(required: &[Method], advertised: &[Method]) -> Vec<Method> {
    required
        .iter()
        .copied()
        .filter(|method| !advertised.contains(method))
        .collect()
}
