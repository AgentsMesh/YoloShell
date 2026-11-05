//! Workspace membership index. Terminals and runs are bound once at creation;
//! the index is a reverse lookup for summaries and counts, not authority.

use super::*;

#[derive(Clone, Copy)]
enum MemberKind {
    Terminal,
    Run,
}

impl WorkspaceStore {
    pub fn bind_terminal(
        &mut self,
        address: WorkspaceAddress,
        terminal_id: Uuid,
    ) -> Result<(), StoreError> {
        self.bind(address, terminal_id, MemberKind::Terminal)
    }

    pub fn bind_run(&mut self, address: WorkspaceAddress, run_id: Uuid) -> Result<(), StoreError> {
        self.bind(address, run_id, MemberKind::Run)
    }

    pub fn members(
        &self,
        address: WorkspaceAddress,
    ) -> Result<Option<(Vec<Uuid>, Vec<Uuid>)>, StoreError> {
        self.check()?;
        Ok(self
            .record
            .find(address)
            .map(|workspace| (workspace.terminals.clone(), workspace.runs.clone())))
    }

    /// Recompute memberships for a page of workspaces under one store check.
    pub fn members_many(
        &self,
        addresses: &[WorkspaceAddress],
    ) -> Result<Vec<Option<(Vec<Uuid>, Vec<Uuid>)>>, StoreError> {
        self.check()?;
        Ok(addresses
            .iter()
            .map(|address| {
                self.record
                    .find(*address)
                    .map(|workspace| (workspace.terminals.clone(), workspace.runs.clone()))
            })
            .collect())
    }

    /// The terminal-to-workspace index used to annotate terminal summaries.
    pub fn terminal_workspaces(&self) -> Result<Vec<(Uuid, WorkspaceAddress)>, StoreError> {
        self.check()?;
        Ok(self
            .record
            .workspaces
            .iter()
            .flat_map(|workspace| {
                workspace
                    .terminals
                    .iter()
                    .map(move |terminal| (*terminal, workspace.address))
            })
            .collect())
    }

    fn bind(
        &mut self,
        address: WorkspaceAddress,
        member: Uuid,
        kind: MemberKind,
    ) -> Result<(), StoreError> {
        self.check()?;
        if member.is_nil() {
            return Err(PublicError::InvalidRequest.into());
        }
        let current = self
            .record
            .find(address)
            .cloned()
            .ok_or(PublicError::NotFound)?;
        let holds = |workspace: &StoredWorkspace| match kind {
            MemberKind::Terminal => workspace.terminals.contains(&member),
            MemberKind::Run => workspace.runs.contains(&member),
        };
        if holds(&current) {
            return Ok(());
        }
        if self
            .record
            .workspaces
            .iter()
            .any(|workspace| workspace.address != address && holds(workspace))
        {
            return Err(PublicError::Conflict.into());
        }
        let mut next = current;
        match kind {
            MemberKind::Terminal => next.terminals.push(member),
            MemberKind::Run => next.runs.push(member),
        }
        match kind {
            MemberKind::Terminal => next.terminals.sort_unstable(),
            MemberKind::Run => next.runs.sort_unstable(),
        }
        self.commit(next)
    }
}
