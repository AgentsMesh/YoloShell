use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct StartRequest {
    pub config_source: PathBuf,
    pub broker_state: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ProfileAuthorityRequest {
    pub document: PathBuf,
    pub lock: PathBuf,
    pub expected_sha256: String,
}

#[derive(Clone, Debug)]
pub struct RetireRequest {
    pub broker_state: PathBuf,
    pub expected_host: String,
    pub authority_document: PathBuf,
    pub authority_lock: PathBuf,
    pub expected_mutation_id: uuid::Uuid,
}
