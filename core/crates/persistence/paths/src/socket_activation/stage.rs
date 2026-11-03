use super::*;

pub(super) struct StageEntry {
    pub(super) directory: PrivateDirectory,
    pub(super) name: CString,
    path: PathBuf,
    identity: Option<SocketIdentity>,
    expected_links: Option<u64>,
    _lease: socket_stage::Lease,
    armed: bool,
}

impl StageEntry {
    pub(super) async fn vacant(target: &SocketTarget, kind: socket_stage::Kind) -> Result<Self> {
        let lease = socket_stage::Lease::acquire(&target.directory, &target.stage).await?;
        clean_residue(target)?;
        let directory = target.directory.try_clone()?;
        let (name, path) = target.stage.stage(kind)?;
        Ok(Self {
            directory,
            name,
            path,
            identity: None,
            expected_links: None,
            _lease: lease,
            armed: true,
        })
    }

    pub(super) async fn link(
        target: &SocketTarget,
        identity: SocketIdentity,
        kind: socket_stage::Kind,
    ) -> Result<Self> {
        let mut alias = Self::vacant(target, kind).await?;
        anyhow::ensure!(
            socket_matches_at(
                &target.directory,
                &target.name,
                true,
                identity,
                crate::paths::constants::SOCKET_ENDPOINT_LINK_COUNT,
            )?,
            PROBE_LINK_COUNT_ERROR
        );
        link_at(target, &alias)?;
        anyhow::ensure!(
            socket_matches_at(
                &alias.directory,
                &alias.name,
                false,
                identity,
                crate::paths::constants::SOCKET_PROBE_LINK_COUNT,
            )? && socket_matches_at(
                &target.directory,
                &target.name,
                true,
                identity,
                crate::paths::constants::SOCKET_PROBE_LINK_COUNT,
            )?,
            PROBE_ALIAS_CHANGED_ERROR
        );
        alias.identity = Some(identity);
        alias.expected_links = Some(crate::paths::constants::SOCKET_PROBE_LINK_COUNT);
        Ok(alias)
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn inspect(&self, private: bool) -> Result<Option<SocketIdentity>> {
        inspect_at(&self.directory, &self.name, private)
    }

    pub(super) fn chmod(&mut self, identity: SocketIdentity) -> Result<()> {
        self.identity = Some(identity);
        self.expected_links = Some(crate::paths::constants::SOCKET_ENDPOINT_LINK_COUNT);
        chmod_at(&self.directory, &self.name)?;
        anyhow::ensure!(self.inspect(true)? == Some(identity), CHMOD_CHANGED_ERROR);
        Ok(())
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StageEntry {
    fn drop(&mut self) {
        if self.armed
            && self
                .identity
                .zip(self.expected_links)
                .is_some_and(|(identity, links)| {
                    socket_matches_at(&self.directory, &self.name, false, identity, links)
                        .unwrap_or(false)
                })
        {
            let _ = unlink_at(&self.directory, &self.name);
            let _ = self.directory.sync();
        }
    }
}

fn clean_residue(target: &SocketTarget) -> Result<()> {
    anyhow::ensure!(
        socket_stage::cleanup_residue(&target.directory, &target.stage, &target.name)?,
        STAGE_RESIDUE_UNPROVEN_ERROR
    );
    Ok(())
}
