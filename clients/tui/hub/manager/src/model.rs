use crate::{constants::*, form::Form};
use uuid::Uuid;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_identity::{FileRootId, HubId};
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_rpc_protocol::Command;

#[derive(Clone, Debug)]
pub struct Operation {
    pub token: Uuid,
    pub command: Command,
}
pub enum Effect {
    Request(Operation),
    Close,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Page {
    Root,
    Peers,
    Peer(HubId),
    Origins,
    FileRoots,
    Exports(HubId),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Page(Page),
    AddPeer,
    EditPeer(HubId),
    RemovePeer(HubId),
    AddOrigin,
    RemoveOrigin(HubId),
    AddExport(HubId),
    EditExport(HubId, HubId),
    RemoveExport(HubId, HubId),
    Listener,
    AddFileRoot,
    EditFileRoot(FileRootId),
    RemoveFileRoot(yolo_hub_file_protocol::FileRootConfiguration),
    Apply,
    Reload,
    Check,
    Close,
}
pub(crate) struct Pending {
    pub token: Uuid,
    pub change: Option<ConfigurationChange>,
    pub reconcile: bool,
}
pub(crate) struct Confirmation {
    pub action: Action,
    pub label: &'static str,
    pub confirm: bool,
}

pub struct Manager {
    pub(crate) epoch: Uuid,
    pub(crate) entry: HubId,
    pub(crate) base: Option<ConfigurationSnapshot>,
    pub(crate) draft: Option<Configuration>,
    pub(crate) page: Page,
    pub(crate) selected: usize,
    pub(crate) form: Option<Form>,
    pub(crate) confirmation: Option<Confirmation>,
    pub(crate) pending: Option<Pending>,
    pub(crate) unresolved: Option<ConfigurationChange>,
    pub(crate) blocked: bool,
    pub(crate) status: String,
}
impl Manager {
    pub fn open(entry: HubId) -> (Self, Operation) {
        let mut manager = Self {
            epoch: Uuid::new_v4(),
            entry,
            base: None,
            draft: None,
            page: Page::Root,
            selected: 0,
            form: None,
            confirmation: None,
            pending: None,
            unresolved: None,
            blocked: false,
            status: LOADING.into(),
        };
        let operation = manager.query(false);
        (manager, operation)
    }
    pub fn dirty(&self) -> bool {
        self.base
            .as_ref()
            .zip(self.draft.as_ref())
            .is_some_and(|(base, draft)| base.state.configuration != *draft)
    }
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn query(&mut self, reconcile: bool) -> Operation {
        let token = Uuid::new_v4();
        self.pending = Some(Pending {
            token,
            change: None,
            reconcile,
        });
        self.status = LOADING.into();
        Operation {
            token,
            command: Command::GetConfiguration {},
        }
    }
    pub(crate) fn proposal(&mut self) -> Option<Operation> {
        if !self.can_apply() {
            return None;
        }
        let base = self.base.as_ref()?;
        let configuration = self.draft.clone()?;
        let change = ConfigurationChange {
            request_id: Uuid::new_v4(),
            expected_revision: base.state.revision,
            configuration,
        };
        if change.validate(base.state.identity).is_err() {
            self.status = INVALID.into();
            return None;
        }
        self.pending = Some(Pending {
            token: change.request_id,
            change: Some(change.clone()),
            reconcile: false,
        });
        self.status = SAVING.into();
        Some(Operation {
            token: change.request_id,
            command: Command::Configure { change },
        })
    }
    pub(crate) fn can_apply(&self) -> bool {
        !self.blocked
            && self.pending.is_none()
            && self.draft.is_some()
            && self
                .base
                .as_ref()
                .is_some_and(|base| self.dirty() || !base.applied)
    }
    pub(crate) fn page(&mut self, page: Page) {
        self.page = page;
        self.selected = 0;
    }
}
