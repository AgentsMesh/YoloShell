use crate::{ClientAdmission, RegistryError, constants::REVISION_STEP};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::sync::watch;
use yolo_hub_client_access_protocol::{AccessConfiguration, ED25519_PUBLIC_KEY_BYTES};

#[derive(Clone)]
pub struct AccessRegistry {
    inner: Arc<Inner>,
}

struct Inner {
    state: Mutex<State>,
    changes: watch::Sender<Option<u64>>,
}

struct State {
    configuration: AccessConfiguration,
    closed: bool,
}

impl AccessRegistry {
    pub fn new(configuration: AccessConfiguration) -> Result<Self, RegistryError> {
        configuration.validate()?;
        let (changes, _) = watch::channel(Some(configuration.revision));
        Ok(Self {
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    configuration,
                    closed: false,
                }),
                changes,
            }),
        })
    }

    pub fn current(&self) -> Result<AccessConfiguration, RegistryError> {
        Ok(self.state()?.configuration.clone())
    }

    /// Call only after the matching durable policy update has committed. An
    /// accepted revision always retires every prior admission, even for equal grants.
    pub fn replace(
        &self,
        expected_revision: u64,
        next: AccessConfiguration,
    ) -> Result<(), RegistryError> {
        next.validate()?;
        let mut state = self.state()?;
        if state.configuration.entry_hub != next.entry_hub {
            return Err(RegistryError::WrongEntry);
        }
        if state.configuration.revision != expected_revision {
            return Err(RegistryError::RevisionConflict);
        }
        let revision = expected_revision
            .checked_add(REVISION_STEP)
            .ok_or(RegistryError::RevisionExhausted)?;
        if next.revision != revision {
            return Err(RegistryError::RevisionConflict);
        }
        state.configuration = next;
        self.inner.changes.send_replace(Some(revision));
        Ok(())
    }

    pub fn close(&self) {
        let mut state = StateGuard {
            state: self
                .inner
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            changes: &self.inner.changes,
        };
        state.closed = true;
        self.inner.changes.send_replace(None);
    }

    /// The transport must verify key possession first, e.g. russh's verified
    /// auth_publickey callback. A key offer or caller-provided key is not proof.
    pub fn admit_verified_key(
        &self,
        key: &[u8; ED25519_PUBLIC_KEY_BYTES],
        now_ms: u64,
    ) -> Result<ClientAdmission, RegistryError> {
        let state = self.state()?;
        let grant = state
            .configuration
            .clients
            .iter()
            .find(|grant| grant.identity.public_key == *key)
            .ok_or(RegistryError::Unauthorized)?;
        grant.validate_at(now_ms)?;
        Ok(ClientAdmission::new(
            grant.clone(),
            state.configuration.entry_hub,
            state.configuration.revision,
            self.inner.changes.subscribe(),
        ))
    }

    fn state(&self) -> Result<StateGuard<'_>, RegistryError> {
        let state = self.inner.state.lock().map_err(|_| {
            self.inner.changes.send_replace(None);
            RegistryError::Unavailable
        })?;
        if state.closed {
            return Err(RegistryError::Closed);
        }
        Ok(StateGuard {
            state,
            changes: &self.inner.changes,
        })
    }
}

/// Revoke before releasing a panicking policy operation's lock. Waiting streams
/// must not depend on another caller discovering the poisoned mutex later.
struct StateGuard<'a> {
    state: MutexGuard<'a, State>,
    changes: &'a watch::Sender<Option<u64>>,
}

impl Deref for StateGuard<'_> {
    type Target = State;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for StateGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl Drop for StateGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.state.closed = true;
            self.changes.send_replace(None);
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/poison.rs"]
mod poison_tests;
