use super::*;

pub(crate) const RESERVED_CREATION_LAUNCH_GENERATION: u64 = 0;
const MAX_CREATION_INPUT_BYTES: usize = 65_536;
pub(crate) const INITIAL_CREATION_LAUNCH_GENERATION: u64 = 1;
pub(crate) const CREATION_LAUNCH_GENERATION_EXHAUSTED: &str =
    "creation launch generation exhausted";
pub(crate) const CREATION_LAUNCH_GENERATION_INCONSISTENT: &str =
    "creation phase and launch generation are inconsistent";
pub(crate) const CREATION_LIVE_RECEIPT_PHASE_CONFLICT: &str =
    "live creation receipt conflicts with active creation phase";
pub(crate) const CREATION_SURVIVOR_PHASE_CONFLICT: &str =
    "only a ready creation can become a survivor";
pub(in crate::host) const CLEANUP_FINALIZED_UNAUTHORIZED: &str =
    "finalized creation is not authorized for cleanup";
pub(in crate::host) const CLEANUP_SURVIVOR_ARCHIVE_UNAUTHORIZED: &str =
    "survivor creation is not authorized for cleanup archive";
pub(in crate::host) const CLEANUP_READY_TRANSITION_INVARIANT: &str =
    "ready cleanup was transitioned";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::host) struct CreationSpec {
    pub(crate) request_fingerprint: String,
    pub(crate) name: Option<String>,
    pub(in crate::host) cwd: PathBuf,
    pub(crate) cols: Option<u16>,
    pub(crate) rows: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) initial_input: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) context: Option<crate::model::TerminalCallerContext>,
}

impl CreationSpec {
    pub(crate) fn resolve(
        request: &CreateTerminal,
        request_fingerprint: String,
    ) -> std::result::Result<Self, ErrorBody> {
        let cwd = resolve_cwd(
            &request.cwd,
            std::env::var_os(crate::agent::constants::HOME_ENV)
                .as_deref()
                .map(Path::new),
        )
        .map_err(|error| {
            ErrorBody::new(
                crate::protocol::error_code::INVALID_DIRECTORY,
                error.to_string(),
                false,
            )
        })?;
        if request
            .initial_input
            .as_ref()
            .is_some_and(|input| input.is_empty() || input.len() > MAX_CREATION_INPUT_BYTES)
        {
            return Err(ErrorBody::invalid("initial input is invalid"));
        }
        if request.context.as_ref().is_some_and(|context| {
            context.hub_id.is_nil()
                || context
                    .workspace_id
                    .is_some_and(|workspace| workspace.is_nil())
        }) {
            return Err(ErrorBody::invalid("caller context is invalid"));
        }
        Ok(Self {
            request_fingerprint,
            name: request.name.clone(),
            cwd,
            cols: request.cols,
            rows: request.rows,
            initial_input: request.initial_input.clone(),
            context: request.context,
        })
    }

    pub(crate) fn fingerprint(request: &CreateTerminal) -> std::result::Result<String, ErrorBody> {
        serde_json::to_vec(request)
            .map(|bytes| yolo_crypto::sha256_hex(&bytes))
            .map_err(|error| ErrorBody::invalid(error.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::host) enum CreationPhase {
    Reserved,
    Launching,
    SpawnFailed,
    Ready,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::host) enum CreationCompletion {
    Finalized,
    Archived,
    Survivor,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::host) struct CreationLaunchIdentity {
    #[serde(with = "crate::serde_uuid")]
    pub(in crate::host) runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(in crate::host) worker_id: Uuid,
    pub(in crate::host) auth_token: String,
}

impl CreationLaunchIdentity {
    pub(crate) fn new() -> Self {
        Self {
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            auth_token: format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::host) struct CreationBinding {
    pub(crate) creation_token_digest: String,
    #[serde(flatten)]
    pub(in crate::host) result: CreateTerminalResult,
    pub(in crate::host) spec: CreationSpec,
    pub(crate) phase: CreationPhase,
    pub(crate) launch_generation: u64,
    pub(crate) launch: Option<CreationLaunchIdentity>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::host) struct CreationLedger {
    pub(crate) schema_version: u16,
    pub(in crate::host) bindings: Vec<CreationBinding>,
    #[serde(skip)]
    pub(crate) poisoned: bool,
    #[serde(skip)]
    pub(crate) entry: Option<Arc<crate::paths::private_file::PrivateFileEntry>>,
}

impl PartialEq for CreationLedger {
    fn eq(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && self.bindings == other.bindings
            && self.poisoned == other.poisoned
    }
}

impl Eq for CreationLedger {}

impl Default for CreationLedger {
    fn default() -> Self {
        Self {
            schema_version: CREATION_LEDGER_SCHEMA_VERSION,
            bindings: Vec::new(),
            poisoned: false,
            entry: None,
        }
    }
}

pub(crate) fn creation_token_digest(token: Uuid) -> String {
    yolo_crypto::sha256_hex(token.as_bytes())
}

pub(crate) fn valid_creation_digest(value: &str) -> bool {
    valid_lower_hex(value, SHA256_HEX_LENGTH)
}

pub(in crate::host) fn valid_lower_hex(value: &str, expected_length: usize) -> bool {
    value.len() == expected_length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn valid_creation_cwd(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .as_os_str()
            .as_encoded_bytes()
            .split(|byte| *byte == std::path::MAIN_SEPARATOR as u8)
            .any(|component| component == b"." || component == b"..")
}
