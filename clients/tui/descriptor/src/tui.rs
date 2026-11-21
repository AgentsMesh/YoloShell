use yolo_builtin_client_contract::BuiltinClientDescriptor;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Method};
use yolo_plugin_api::{ClientApiContract, ClientCapability, ClientFeatureRequirement, Platform};

pub const TUI_ID: &str = "yolo.tui";

const TUI_REQUIRED_METHODS: &[Method] = &[
    Method::Hello,
    Method::List,
    Method::Get,
    Method::Attach,
    Method::ListLayouts,
    Method::GetLayout,
    Method::UpdateLayout,
];

pub fn tui_api() -> ClientApiContract {
    let required = TUI_REQUIRED_METHODS.to_vec();
    let optional: Vec<Method> = yolo_hub_rpc_descriptor::compiled_methods()
        .iter()
        .copied()
        .filter(|method| !required.contains(method))
        .collect();
    let features: Vec<ClientFeatureRequirement> = ClientFeatureRequirement::all()
        .into_iter()
        .filter(|feature| {
            !matches!(
                feature.capability,
                ClientCapability::FileAccess | ClientCapability::RemoteSetup
            )
        })
        .map(|mut feature| {
            if feature.capability == ClientCapability::AgentObservation {
                feature.methods = vec![Method::ListAgents];
            }
            feature
        })
        .collect();
    ClientApiContract::new(HUB_PROTOCOL_VERSION, required, optional, features)
}

pub fn tui_descriptor() -> BuiltinClientDescriptor {
    BuiltinClientDescriptor::yolo_image(TUI_ID, Platform::Unix, tui_api())
}
