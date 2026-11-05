use crate::{
    constants::*,
    fields::FieldId,
    form::{Form, FormKind},
    wire_fields::*,
};
use anyhow::{Context, Result, ensure};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::{Configuration, ExportPolicy, Peer, PeerAddress};
use yolo_hub_routing::Right;

impl Form {
    pub fn commit(&self, draft: &Configuration, local: PeerIdentity) -> Result<Configuration> {
        let mut next = draft.clone();
        match self.kind {
            FormKind::FileRoot { id, existing } => {
                self.commit_file_root(&mut next, id, existing)?
            }
            FormKind::Peer(previous) => {
                let identity = identity(&self.fields)?;
                ensure!(
                    previous.is_none_or(|previous| previous == identity),
                    INVALID
                );
                let profile = text(&self.fields, FieldId::Profile)?;
                let socket = text(&self.fields, FieldId::Socket)?;
                ensure!(profile.is_empty() == socket.is_empty(), INVALID);
                let dial = if profile.is_empty() {
                    None
                } else {
                    Some(PeerAddress::ssh(profile, socket.into())?)
                };
                let exports = previous
                    .map(|previous| {
                        next.peer(previous.hub())
                            .map(|peer| peer.exports.clone())
                            .context(INVALID)
                    })
                    .transpose()?
                    .unwrap_or_default();
                let peer = Peer {
                    identity,
                    dial,
                    consume: flag(&self.fields, FieldId::Import)?,
                    exports,
                };
                if let Some(previous) = previous {
                    let current = next
                        .peers
                        .iter_mut()
                        .find(|peer| peer.identity == previous)
                        .context(INVALID)?;
                    *current = peer;
                } else {
                    next.peers.push(peer);
                }
            }
            FormKind::Origin => {
                let identity = identity(&self.fields)?;
                ensure!(
                    !next
                        .trusted_origins
                        .iter()
                        .any(|origin| origin.hub() == identity.hub()),
                    INVALID
                );
                next.trusted_origins.push(identity);
            }
            FormKind::Export { peer, original } => {
                let origin = text(&self.fields, FieldId::Origin)?.parse()?;
                ensure!(original.is_none_or(|previous| previous == origin), INVALID);
                let mut rights = vec![Right::Observe];
                for (field, right) in [
                    (FieldId::ReadTerminal, Right::TerminalRead),
                    (FieldId::WriteTerminal, Right::TerminalWrite),
                    (FieldId::Lifecycle, Right::Lifecycle),
                    (FieldId::FileRead, Right::FileRead),
                    (FieldId::AgentControl, Right::AgentControl),
                    (FieldId::ConversationRead, Right::ConversationRead),
                    (FieldId::PresentationRead, Right::PresentationRead),
                    (FieldId::PresentationWrite, Right::PresentationWrite),
                ] {
                    if flag(&self.fields, field)? {
                        rights.push(right);
                    }
                }
                let policy = ExportPolicy {
                    origin,
                    rights: rights.into_iter().collect(),
                    remaining_reexports: number(&self.fields, FieldId::Reexports)?,
                };
                let target = next
                    .peers
                    .iter_mut()
                    .find(|target| target.identity.hub() == peer)
                    .context(INVALID)?;
                if let Some(original) = original {
                    *target
                        .exports
                        .iter_mut()
                        .find(|policy| policy.origin == original)
                        .context(INVALID)? = policy;
                } else {
                    target.exports.push(policy);
                }
            }
        }
        next.validate(local)?;
        Ok(next)
    }
}
