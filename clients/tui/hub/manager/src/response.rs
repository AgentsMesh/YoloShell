use crate::{Manager, constants::*};
use uuid::Uuid;
use yolo_hub_management_client::ManagementError;
use yolo_hub_management_protocol::{
    CONFIGURATION_REVISION_STEP, ConfigurationChange, ConfigurationSnapshot,
};

impl Manager {
    pub fn complete(
        &mut self,
        token: Uuid,
        result: Result<ConfigurationSnapshot, ManagementError>,
    ) {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.token != token)
        {
            return;
        }
        let pending = self.pending.take().unwrap();
        self.epoch = Uuid::new_v4();
        match result {
            Ok(snapshot)
                if snapshot.validate().is_ok()
                    && snapshot.state.identity.hub() == self.entry
                    && self.base.as_ref().is_none_or(|base| {
                        base.state.identity == snapshot.state.identity
                            && (snapshot.state.revision > base.state.revision
                                || snapshot.state == base.state)
                    }) =>
            {
                let expected = pending.change.as_ref().or_else(|| {
                    pending
                        .reconcile
                        .then_some(self.unresolved.as_ref())
                        .flatten()
                });
                if expected.is_some_and(|change| !matches_change(&snapshot, change)) {
                    self.blocked = true;
                    self.unresolved = pending.change.or_else(|| self.unresolved.take());
                    self.status = UNCONFIRMED.into();
                    return;
                }
                self.draft = Some(snapshot.state.configuration.clone());
                self.status = if snapshot.applied { READY } else { INACTIVE }.into();
                self.base = Some(snapshot);
                self.blocked = false;
                self.unresolved = None;
                self.page(crate::model::Page::Root);
            }
            Ok(_) => {
                self.blocked = true;
                if pending.change.is_some() {
                    self.unresolved = pending.change;
                }
                self.status = if self.unresolved.is_some() {
                    UNKNOWN.into()
                } else {
                    ManagementError::Unavailable.to_string()
                };
            }
            Err(error) => {
                self.blocked = true;
                if let Some(change) = pending.change {
                    if error == ManagementError::OutcomeUnknown {
                        self.unresolved = Some(change);
                    }
                }
                self.status = error.to_string();
            }
        }
    }
    pub fn not_sent(&mut self, token: Uuid) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.token == token)
        {
            self.pending = None;
            self.epoch = Uuid::new_v4();
            self.status = ManagementError::Unavailable.to_string();
        }
    }
}
fn matches_change(snapshot: &ConfigurationSnapshot, change: &ConfigurationChange) -> bool {
    snapshot.applied
        && snapshot.state.last_change == change.request_id
        && Some(snapshot.state.revision)
            == change
                .expected_revision
                .checked_add(CONFIGURATION_REVISION_STEP)
        && snapshot.state.configuration == change.configuration
}
