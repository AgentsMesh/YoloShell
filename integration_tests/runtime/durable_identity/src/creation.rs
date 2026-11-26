use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, CreateShellResult, TERMINAL_PROTOCOL_SCHEMA_VERSION,
    SecretCapability, TerminalSize,
};
use yolo_paths::directory::PrivateDirectory;
use yolo_tui_creation_journal::{CreationIntent, CreationJournal, PendingFinalization};

pub const DIRECTORY: &str = "client";
const CHANGED: &str = "TUI creation evidence changed or granted duplicate creation authority";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationEvidence {
    intent: CreationIntent,
    pending: PendingFinalization,
}

impl CreationEvidence {
    pub fn seed(root: &Path, entry: HubId) -> Result<Self> {
        let parent = PrivateDirectory::ensure(&root.join(DIRECTORY))?;
        let mut journal = CreationJournal::open(parent, entry)?;
        let origin = HubId::new();
        let intent = CreationIntent::new(origin, request(root)?)?;
        let pending = PendingFinalization::new(
            origin,
            request(root)?,
            CreateShellResult {
                address: TerminalAddress {
                    hub_id: origin,
                    terminal_id: TerminalId::new(),
                },
                creation_capability: SecretCapability::generate(),
            },
        )?;
        ensure!(journal.retain_intent(intent.clone())?, CHANGED);
        ensure!(journal.retain_confirmed(pending.clone())?, CHANGED);
        Ok(Self { intent, pending })
    }

    pub fn verify(&self, root: &Path, entry: HubId) -> Result<()> {
        let parent = PrivateDirectory::open(&root.join(DIRECTORY))?;
        let mut journal = CreationJournal::open(parent, entry)?;
        ensure!(journal.intents()? == [self.intent.clone()], CHANGED);
        ensure!(journal.pending()? == [self.pending.clone()], CHANGED);
        ensure!(!journal.retain_intent(self.intent.clone())?, CHANGED);
        ensure!(!journal.retain_confirmed(self.pending.clone())?, CHANGED);
        journal.verify()
    }
}

fn request(root: &Path) -> Result<CreateShellRequest> {
    Ok(CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        creation_token: Uuid::new_v4(),
        name: None,
        cwd: root.to_string_lossy().into_owned(),
        size: TerminalSize::default(),
    })?)
}
