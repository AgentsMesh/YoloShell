use crate::{
    ListenerBindings,
    constants::*,
    model::{Entry, Record},
    persistent,
};
use anyhow::{Result, ensure};
use std::net::SocketAddr;
use yolo_hub_management_protocol::ConfigurationState;

impl ListenerBindings {
    /// Publish the first allocated endpoint while its real listener FD remains held.
    pub fn establish(
        &mut self,
        configuration: &ConfigurationState,
        address: SocketAddr,
    ) -> Result<()> {
        let mut record = self.retain(self.read())?;
        ensure!(
            persistent(configuration.configuration.listen)
                && record.current.configuration == *configuration
                && record.current.address.is_none()
                && record.pending.is_none(),
            CONFLICT
        );
        record.current.address = Some(address);
        self.write(&record)
    }

    /// Keep the old binding usable until the exact configuration CAS commits.
    pub fn prepare(
        &mut self,
        current: &ConfigurationState,
        next: &ConfigurationState,
        listener_address: Option<SocketAddr>,
    ) -> Result<()> {
        let record = self.retain(self.read())?;
        let current = record.selected(current)?.clone();
        current.ready()?;
        let pending = Entry {
            configuration: next.clone(),
            address: persistent(next.configuration.listen)
                .then_some(listener_address)
                .flatten(),
        };
        let record = Record {
            current,
            pending: Some(pending),
            ..record
        };
        self.write(&record)
    }

    fn write(&mut self, record: &Record) -> Result<()> {
        record.validate(self.root, self.peer)?;
        let result = self.namespace.write_json_expected_with_commit_fence(
            &self.path,
            Some(self.file),
            record,
            || self.read().map(|_| ()),
        );
        self.file = self.retain(result)?;
        self.retain(self.read())?;
        Ok(())
    }
}
