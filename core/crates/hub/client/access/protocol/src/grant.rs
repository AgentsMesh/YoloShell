use crate::{
    AccessError, ClientIdentity, MAX_CLIENT_ORIGINS, OriginAccess, constants::INVALID_EXPIRY,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use yolo_hub_identity::HubId;
use yolo_hub_routing::Rights;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "GrantWire")]
pub struct ClientGrant {
    pub identity: ClientIdentity,
    pub origins: Vec<OriginAccess>,
    pub expires_at_ms: u64,
}

impl ClientGrant {
    pub fn validate(&self) -> Result<(), AccessError> {
        self.identity.validate()?;
        if self.origins.is_empty() || self.expires_at_ms == INVALID_EXPIRY {
            return Err(AccessError::Grant);
        }
        if self.origins.len() > MAX_CLIENT_ORIGINS {
            return Err(AccessError::Capacity);
        }
        let mut origins = BTreeSet::new();
        for access in &self.origins {
            access.validate()?;
            if !origins.insert(access.origin) {
                return Err(AccessError::DuplicateOrigin);
            }
        }
        Ok(())
    }

    pub fn validate_at(&self, now_ms: u64) -> Result<(), AccessError> {
        self.validate()?;
        if self.expires_at_ms <= now_ms {
            Err(AccessError::Expired)
        } else {
            Ok(())
        }
    }

    /// Rights are independent. Callers must also validate expiry at admission.
    pub fn allows(&self, origin: HubId, required: &Rights) -> bool {
        !required.is_empty()
            && self.validate().is_ok()
            && self
                .origins
                .iter()
                .any(|access| access.origin == origin && access.rights.allows(required))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrantWire {
    identity: ClientIdentity,
    origins: Vec<OriginAccess>,
    expires_at_ms: u64,
}

impl TryFrom<GrantWire> for ClientGrant {
    type Error = AccessError;

    fn try_from(wire: GrantWire) -> Result<Self, Self::Error> {
        let grant = Self {
            identity: wire.identity,
            origins: wire.origins,
            expires_at_ms: wire.expires_at_ms,
        };
        grant.validate()?;
        Ok(grant)
    }
}
