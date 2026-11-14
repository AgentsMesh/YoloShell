use crate::ServerOptions;
use yolo_broker_startup::StartRequest;

pub fn from_request(request: &StartRequest) -> ServerOptions {
    ServerOptions {
        config: request.config_source.clone(),
        state_dir: request.broker_state.clone(),
        expected_revision: None,
        expected_anchor: None,
        expected_authority: None,
        expected_startup_deadline: None,
        authority_document: None,
        authority_lock: None,
        expected_profile_sha256: None,
        identity_passphrases_stdin: false,
    }
}
