use super::{SocketDirectory, restore_between, with_restore};
use crate::remote_broker::constants::*;
use crate::remote_broker::socket_directory_io::rename_exclusive;
use crate::remote_broker::socket_identity::SocketIdentity;
use anyhow::{Result, ensure};
use std::ffi::CStr;

impl SocketDirectory {
    pub(in crate::remote_broker) fn publish_expected_from(
        &self,
        source: &Self,
        source_name: &CStr,
        target_name: &CStr,
        expected: SocketIdentity,
    ) -> Result<()> {
        self.publish_expected_from_with(source, source_name, target_name, expected, || {
            source.directory.sync()?;
            self.directory.sync()
        })
    }

    fn publish_expected_from_with<F>(
        &self,
        source: &Self,
        source_name: &CStr,
        target_name: &CStr,
        expected: SocketIdentity,
        commit: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        ensure!(
            source.identity(source_name)? == Some(expected),
            BROKER_SOCKET_CHANGED
        );
        ensure!(
            rename_exclusive(
                source.descriptor(),
                source_name,
                self.descriptor(),
                target_name,
            )?,
            BROKER_SOCKET_PUBLISH_TARGET_EXISTS
        );
        let identity = self.identity(target_name);
        let verified = matches!(&identity, Ok(Some(actual)) if *actual == expected);
        let outcome = if verified {
            commit()
        } else {
            Err(identity
                .map(|_| anyhow::anyhow!(BROKER_SOCKET_PUBLISH_CHANGED))
                .unwrap_or_else(|error| error.context(BROKER_SOCKET_PUBLISH_CHANGED)))
        };
        if let Err(error) = outcome {
            let restore = restore_between(self, target_name, source, source_name);
            return Err(with_restore(
                error,
                restore,
                BROKER_SOCKET_PUBLISH_RESTORE_FAILED,
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn publish_expected_from_with_commit<F>(
        &self,
        source: &Self,
        source_name: &CStr,
        target_name: &CStr,
        expected: SocketIdentity,
        commit: F,
    ) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.publish_expected_from_with(source, source_name, target_name, expected, commit)
    }
}
