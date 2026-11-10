use crate::{
    Store, StoredState,
    constants::*,
    record::{Member, Record},
};
use anyhow::{Context, Result, ensure};
use yolo_hub_identity::ConnectorId;

impl<T: StoredState> Store<T> {
    pub(crate) fn prepare(&mut self, initial: Record<T>) -> Result<()> {
        self.check_index()?;
        self.check_inventory()?;
        initial.validate(self.identity)?;
        self.index.require_available_seed(&initial.seed)?;
        let mut next = self.index.clone();
        next.members.push(Member::Preparing { initial });
        self.write_index(next, None)
    }

    pub(crate) fn stage_member(&mut self, id: ConnectorId) -> Result<()> {
        self.check_index()?;
        let Some(Member::Preparing { initial }) = self.index.member(id) else {
            anyhow::bail!(NOT_PREPARING)
        };
        if self.member_identities.contains_key(&id) {
            // A recovered file must still be exactly the durable initial record.
            self.read_member(id)?;
            return Ok(());
        }
        let identity = self.namespace.write_json_expected_with_commit_fence(
            &self.member_path(id),
            None,
            initial,
            || self.check_index(),
        )?;
        self.member_identities.insert(id, identity);
        self.check_index()?;
        self.read_member(id)?;
        Ok(())
    }

    pub(crate) fn finish(&mut self, id: ConnectorId) -> Result<()> {
        self.check_index()?;
        let record = self.read_member(id)?;
        ensure!(
            matches!(self.index.member(id), Some(Member::Preparing { .. })),
            NOT_PREPARING
        );
        let expected = *self.member_identities.get(&id).context(MEMBER_MISSING)?;
        let mut next = self.index.clone();
        let member = next
            .members
            .iter_mut()
            .find(|member| member.id() == id)
            .context(NOT_PREPARING)?;
        *member = Member::Ready {
            connector_id: id,
            request_id: record.seed.request_id,
            seed_digest: record.seed.digest()?,
        };
        self.write_index(next, Some((id, expected)))?;
        self.read_member(id)?;
        self.check_inventory()?;
        Ok(())
    }
}
