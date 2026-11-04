use crate::{
    fields::{Field, FieldId, Value},
    wire_fields::key_text,
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::{DialTransport, ExportPolicy, Peer};
use yolo_hub_identity::{FileRootId, HubId};
use yolo_hub_routing::Right;

#[derive(Clone, Copy, Debug)]
pub(crate) enum FormKind {
    Peer(Option<PeerIdentity>),
    Origin,
    Export {
        peer: HubId,
        original: Option<HubId>,
    },
    Listener,
    FileRoot {
        id: FileRootId,
        existing: bool,
    },
}
pub(crate) struct Form {
    pub kind: FormKind,
    pub fields: Vec<Field>,
    pub focused: usize,
    pub original_file_root: Option<yolo_hub_file_protocol::FileRootConfiguration>,
}
impl Form {
    pub(crate) fn new(kind: FormKind, fields: Vec<Field>) -> Self {
        let focused = fields
            .iter()
            .position(|field| !field.read_only)
            .unwrap_or(fields.len());
        Self {
            kind,
            fields,
            focused,
            original_file_root: None,
        }
    }
    pub fn peer(peer: Option<&Peer>) -> Self {
        let identity = peer.map(|peer| peer.identity);
        let read_only = identity.is_some();
        let managed_transport = peer
            .and_then(|peer| peer.dial.as_ref())
            .is_some_and(|dial| matches!(dial.transport, DialTransport::Ssh { .. }));
        Self::new(
            FormKind::Peer(identity),
            vec![
                Field::text(
                    FieldId::Hub,
                    identity.map(|id| id.hub().to_string()).unwrap_or_default(),
                    read_only,
                ),
                Field::text(
                    FieldId::Key,
                    identity.map(key_text).unwrap_or_default(),
                    read_only,
                ),
                Field::text(
                    FieldId::Host,
                    peer.and_then(|peer| peer.dial.as_ref())
                        .map(|dial| dial.host.clone())
                        .unwrap_or_default(),
                    managed_transport,
                ),
                Field::text(
                    FieldId::Port,
                    peer.and_then(|peer| peer.dial.as_ref())
                        .map(|dial| dial.port.to_string())
                        .unwrap_or_default(),
                    managed_transport,
                ),
                Field::flag(FieldId::Import, peer.is_none_or(|peer| peer.consume), false),
            ],
        )
    }
    pub fn origin() -> Self {
        Self::new(
            FormKind::Origin,
            vec![
                Field::text(FieldId::Hub, "", false),
                Field::text(FieldId::Key, "", false),
            ],
        )
    }
    pub fn listener(address: Option<std::net::SocketAddr>) -> Self {
        Self::new(
            FormKind::Listener,
            vec![Field::text(
                FieldId::Listen,
                address
                    .map(|address| address.to_string())
                    .unwrap_or_default(),
                false,
            )],
        )
    }
    pub fn export(peer: HubId, local: HubId, policy: Option<&ExportPolicy>) -> Self {
        let rights = policy
            .map(|policy| policy.rights.clone())
            .unwrap_or_default();
        let allowed = |right| rights.allows(&[right].into_iter().collect());
        Self::new(
            FormKind::Export {
                peer,
                original: policy.map(|policy| policy.origin),
            },
            vec![
                Field::text(
                    FieldId::Origin,
                    policy.map_or(local, |policy| policy.origin).to_string(),
                    policy.is_some(),
                ),
                Field::flag(FieldId::Observe, true, true),
                Field::flag(FieldId::ReadTerminal, allowed(Right::TerminalRead), false),
                Field::flag(FieldId::WriteTerminal, allowed(Right::TerminalWrite), false),
                Field::flag(FieldId::Lifecycle, allowed(Right::Lifecycle), false),
                Field::flag(FieldId::FileRead, allowed(Right::FileRead), false),
                Field::flag(FieldId::AgentControl, allowed(Right::AgentControl), false),
                Field::flag(
                    FieldId::ConversationRead,
                    allowed(Right::ConversationRead),
                    false,
                ),
                Field::flag(
                    FieldId::PresentationRead,
                    allowed(Right::PresentationRead),
                    false,
                ),
                Field::flag(
                    FieldId::PresentationWrite,
                    allowed(Right::PresentationWrite),
                    false,
                ),
                Field {
                    id: FieldId::Reexports,
                    value: Value::Stepper(policy.map_or(0, |policy| policy.remaining_reexports)),
                    read_only: false,
                },
            ],
        )
    }
    pub fn removable(&self) -> bool {
        matches!(
            self.kind,
            FormKind::Export {
                original: Some(_),
                ..
            } | FormKind::FileRoot { existing: true, .. }
        )
    }
    pub fn rows(&self) -> usize {
        self.fields.len() + crate::constants::FORM_COMMANDS + usize::from(self.removable())
    }
    pub fn move_focus(&mut self, forward: bool) {
        let length = self.rows();
        loop {
            self.focused = if forward {
                (self.focused + 1) % length
            } else {
                (self.focused + length - 1) % length
            };
            if self
                .fields
                .get(self.focused)
                .is_none_or(|field| !field.read_only)
            {
                break;
            }
        }
    }
}
