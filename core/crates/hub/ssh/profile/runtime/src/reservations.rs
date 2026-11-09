use crate::constants::*;
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use yolo_hub_ssh_profiles::{SshBrokerProfile, SshProfileConfiguration, SshProfileRef};

#[derive(Default)]
pub(crate) struct Reservations(BTreeMap<SshProfileRef, SshBrokerProfile>);

pub struct ProfileReservation {
    registry: Arc<Mutex<Reservations>>,
    registration: SshBrokerProfile,
}

impl ProfileReservation {
    pub fn registration(&self) -> &SshBrokerProfile {
        &self.registration
    }
    pub(crate) fn belongs_to(&self, owner: &Arc<Mutex<Reservations>>) -> bool {
        Arc::ptr_eq(&self.registry, owner)
    }
}

impl Reservations {
    pub fn reserve(
        registry: &Arc<Mutex<Self>>,
        registration: SshBrokerProfile,
    ) -> Result<ProfileReservation> {
        let mut current = registry.lock().map_err(|_| anyhow::anyhow!(POISONED))?;
        ensure!(!current.0.contains_key(&registration.profile), LAUNCH_BUSY);
        ensure!(
            current.0.len() < yolo_hub_ssh_profiles::MAX_SSH_PROFILES,
            LAUNCH_BUSY
        );
        current
            .0
            .insert(registration.profile.clone(), registration.clone());
        Ok(ProfileReservation {
            registry: registry.clone(),
            registration,
        })
    }
    pub fn permits(&self, configuration: &SshProfileConfiguration) -> bool {
        self.0
            .values()
            .all(|registration| configuration.profiles.contains(registration))
    }
}

impl Drop for ProfileReservation {
    fn drop(&mut self) {
        let mut registry = self
            .registry
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if registry.0.get(&self.registration.profile) == Some(&self.registration) {
            registry.0.remove(&self.registration.profile);
        }
    }
}
