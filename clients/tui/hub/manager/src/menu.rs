use crate::{
    Manager,
    constants::*,
    model::{Action, Page},
};
pub(crate) struct Item {
    pub action: Action,
    pub label: String,
    pub enabled: bool,
}
impl Item {
    fn new(action: Action, label: impl Into<String>) -> Self {
        Self {
            action,
            label: label.into(),
            enabled: true,
        }
    }
}
impl Manager {
    pub(crate) fn items(&self) -> Vec<Item> {
        let Some(draft) = &self.draft else {
            return vec![
                Item::new(Action::Reload, RELOAD),
                Item::new(Action::Close, CLOSE),
            ];
        };
        let mut items = Vec::new();
        match self.page {
            Page::Root => {
                items.push(Item::new(
                    Action::Page(Page::Peers),
                    format!("{CONNECTIONS} ({})", draft.peers.len()),
                ));
                let consumers = self.consumers.as_ref().map_or_else(
                    || CONSUMERS.into(),
                    |list| format!("{CONSUMERS} ({})", list.len()),
                );
                items.push(Item::new(Action::Page(Page::Consumers), consumers));
                items.push(Item::new(
                    Action::Page(Page::Origins),
                    format!("{ORIGINS} ({})", draft.trusted_origins.len()),
                ));
                items.push(Item {
                    action: Action::Page(Page::FileRoots),
                    label: format!("{FILE_ROOTS} ({})", draft.file_roots.len()),
                    enabled: true,
                });
                items.push(Item {
                    action: Action::Apply,
                    label: if !self.dirty() && self.base.as_ref().is_some_and(|base| !base.applied)
                    {
                        ACTIVATE
                    } else {
                        APPLY
                    }
                    .into(),
                    enabled: self.can_apply(),
                });
                if self.unresolved.is_some() {
                    items.push(Item::new(Action::Check, CHECK));
                }
                items.push(Item::new(Action::Reload, RELOAD));
                items.push(Item::new(Action::Close, CLOSE));
            }
            Page::Peers => {
                for peer in &draft.peers {
                    let dial = peer.dial.as_ref().map_or_else(
                        || INBOUND.into(),
                        |dial| format!(
                            "{}:{}",
                            dial.ssh_target().or_else(|| dial.profile_id()).unwrap_or_default(),
                            dial.socket().display()
                        ),
                    );
                    items.push(Item::new(
                        Action::Page(Page::Peer(peer.identity.hub())),
                        format!(
                            "{}  {dial}  {}",
                            peer.identity.hub(),
                            if peer.consume { IMPORT_ON } else { IMPORT_OFF }
                        ),
                    ));
                }
                items.push(Item::new(Action::AddPeer, ADD_PEER));
                items.push(Item::new(Action::Page(Page::Root), BACK));
            }
            Page::Consumers => {
                match &self.consumers {
                    Some(consumers) if consumers.is_empty() => items.push(Item {
                        action: Action::None,
                        label: CONSUMERS_EMPTY.into(),
                        enabled: false,
                    }),
                    Some(consumers) => {
                        for consumer in consumers {
                            items.push(Item {
                                action: Action::None,
                                label: format!(
                                    "{}  {} {LINKS}  {CONNECTED} {}",
                                    consumer.hub,
                                    consumer.links,
                                    age(consumer.connected_since_ms)
                                ),
                                enabled: false,
                            });
                        }
                    }
                    None => {}
                }
                items.push(Item::new(Action::RefreshConsumers, REFRESH_CONSUMERS));
                items.push(Item::new(Action::Page(Page::Root), BACK));
            }
            Page::Peer(hub) => {
                items.push(Item::new(Action::EditPeer(hub), EDIT_PEER));
                items.push(Item::new(Action::Page(Page::Exports(hub)), EXPORTS));
                items.push(Item::new(Action::RemovePeer(hub), REMOVE_PEER));
                items.push(Item::new(Action::Page(Page::Peers), BACK));
            }
            Page::Origins => {
                for origin in &draft.trusted_origins {
                    items.push(Item::new(
                        Action::RemoveOrigin(origin.hub()),
                        format!(
                            "{}  {}",
                            origin.hub(),
                            crate::wire_fields::key_text(*origin)
                        ),
                    ));
                }
                items.push(Item::new(Action::AddOrigin, ADD_ORIGIN));
                items.push(Item::new(Action::Page(Page::Root), BACK));
            }
            Page::FileRoots => {
                for root in &draft.file_roots {
                    items.push(Item::new(
                        Action::EditFileRoot(root.fields().root_id),
                        format!("{}  {}", root.fields().name, root.fields().path),
                    ));
                }
                items.push(Item::new(Action::AddFileRoot, ADD_FILE_ROOT));
                items.push(Item::new(Action::Page(Page::Root), BACK));
            }
            Page::Exports(hub) => {
                if let Some(peer) = draft.peer(hub) {
                    for policy in &peer.exports {
                        items.push(Item::new(
                            Action::EditExport(hub, policy.origin),
                            format!(
                                "{}  {REEXPORTS}: {}",
                                policy.origin, policy.remaining_reexports
                            ),
                        ));
                    }
                }
                items.push(Item::new(Action::AddExport(hub), ADD_EXPORT));
                items.push(Item::new(Action::Page(Page::Peer(hub)), BACK));
            }
        }
        items
    }
}

fn age(since_ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(since_ms);
    let seconds = now.saturating_sub(since_ms) / 1_000;
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3_599 => format!("{}m", seconds / 60),
        _ => format!("{}h", seconds / 3_600),
    }
}
