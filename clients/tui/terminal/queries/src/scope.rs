use uuid::Uuid;

/// A caller-created writer/connection epoch, never a writer secret or authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueryScope(Uuid);

impl QueryScope {
    pub fn new(epoch: Uuid) -> Option<Self> {
        (!epoch.is_nil()).then_some(Self(epoch))
    }
}
