use crate::{Seed, StoredState, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;
use yolo_crypto::Sha256Digest;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::ConnectorId;
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record<T> {
    pub seed: Seed,
    pub state: T,
}

impl<T: StoredState> Record<T> {
    pub fn validate(&self, identity: PeerIdentity) -> Result<()> {
        self.seed.validate(identity)?;
        self.state.validate(&self.seed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "registration_phase",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum Member<T> {
    Preparing {
        initial: Record<T>,
    },
    Ready {
        connector_id: ConnectorId,
        #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
        request_id: Uuid,
        seed_digest: Sha256Digest,
    },
}

impl<T> Member<T> {
    pub fn id(&self) -> ConnectorId {
        match self {
            Self::Preparing { initial } => initial.seed.connector_id,
            Self::Ready { connector_id, .. } => *connector_id,
        }
    }
    pub fn request_id(&self) -> Uuid {
        match self {
            Self::Preparing { initial } => initial.seed.request_id,
            Self::Ready { request_id, .. } => *request_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Index<T> {
    pub schema_version: u16,
    pub identity: PeerIdentity,
    pub directory: DurableObjectIdentity,
    pub members_directory: DurableObjectIdentity,
    pub members: Vec<Member<T>>,
}

impl<T: StoredState> Index<T> {
    pub fn validate(
        &self,
        directory: DurableObjectIdentity,
        members_directory: DurableObjectIdentity,
        identity: PeerIdentity,
    ) -> Result<()> {
        ensure!(self.schema_version == SCHEMA_VERSION, INVALID_INDEX);
        ensure!(
            self.identity == identity
                && self.directory == directory
                && self.members_directory == members_directory,
            WRONG_IDENTITY
        );
        ensure!(self.members.len() <= MAX_CONNECTORS, CAPACITY);
        let mut ids = BTreeSet::new();
        let mut requests = BTreeSet::new();
        for member in &self.members {
            ensure!(
                ids.insert(member.id()) && requests.insert(member.request_id()),
                DUPLICATE
            );
            ensure!(!member.request_id().is_nil(), INVALID_INDEX);
            if let Member::Preparing { initial } = member {
                initial.validate(identity)?;
            }
        }
        Ok(())
    }
    pub fn member(&self, id: ConnectorId) -> Option<&Member<T>> {
        self.members.iter().find(|member| member.id() == id)
    }
    pub fn require_available_seed(&self, seed: &Seed) -> Result<()> {
        ensure!(self.members.len() < MAX_CONNECTORS, CAPACITY);
        ensure!(
            !self
                .members
                .iter()
                .any(|member| member.id() == seed.connector_id
                    || member.request_id() == seed.request_id),
            DUPLICATE
        );
        Ok(())
    }
}
