use crate::{Manager, constants::*, model::Action};
use yolo_hub_identity::HubId;

impl Manager {
    pub(crate) fn remove(&mut self, action: Action) {
        let Some(mut draft) = self.draft.clone() else {
            return;
        };
        let destination = if matches!(&action, Action::RemoveFileRoot(_)) {
            crate::model::Page::FileRoots
        } else {
            crate::model::Page::Root
        };
        match action {
            Action::RemoveFileRoot(previous) => {
                let Some(index) = draft
                    .file_roots
                    .iter()
                    .position(|root| root.fields().root_id == previous.fields().root_id)
                else {
                    self.status = INVALID.into();
                    return;
                };
                if draft.file_roots[index] != previous {
                    self.status = INVALID.into();
                    return;
                }
                draft.file_roots.remove(index);
            }
            Action::RemovePeer(hub) => {
                draft.peers.retain(|peer| peer.identity.hub() != hub);
                prune(&mut draft, hub);
            }
            Action::RemoveOrigin(hub) => {
                draft.trusted_origins.retain(|origin| origin.hub() != hub);
                prune(&mut draft, hub);
            }
            Action::RemoveExport(peer, origin) => {
                if let Some(peer) = draft
                    .peers
                    .iter_mut()
                    .find(|target| target.identity.hub() == peer)
                {
                    peer.exports.retain(|policy| policy.origin != origin);
                }
            }
            _ => return,
        }
        let Some(base) = &self.base else {
            return;
        };
        if draft.validate(base.state.identity).is_err() {
            self.status = INVALID.into();
            return;
        }
        self.draft = Some(draft);
        self.form = None;
        self.draft_status();
        self.page(destination);
    }
    pub(crate) fn save_form(&mut self) {
        let Some(((form, draft), base)) = self
            .form
            .as_ref()
            .zip(self.draft.as_ref())
            .zip(self.base.as_ref())
        else {
            return;
        };
        match form.commit(draft, base.state.identity) {
            Ok(draft) => {
                self.draft = Some(draft);
                self.form = None;
                self.draft_status();
            }
            Err(_) => self.status = INVALID.into(),
        }
    }

    fn draft_status(&mut self) {
        self.status = if self.dirty() {
            DRAFT
        } else if self.base.as_ref().is_some_and(|base| base.applied) {
            READY
        } else {
            INACTIVE
        }
        .into();
    }
}
fn prune(configuration: &mut yolo_hub_federation_settings::Configuration, origin: HubId) {
    if !configuration.pins().iter().any(|pin| pin.hub() == origin) {
        for peer in &mut configuration.peers {
            peer.exports.retain(|policy| policy.origin != origin);
        }
    }
}
