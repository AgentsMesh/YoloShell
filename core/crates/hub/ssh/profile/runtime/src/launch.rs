use crate::{
    SshProfileRuntime,
    constants::*,
    reservations::{ProfileReservation, Reservations},
};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;
use tokio::time::Instant;
use yolo_broker_hub_protocol::{BrokerHubConnectionBinding, BrokerHubProfile};
use yolo_hub_broker_launcher::Launch;
use yolo_hub_ssh_profiles::{SshBrokerProfile, SshProfileRef};
use yolo_remote_broker::{BrokerClient, BrokerPaths};
use zeroize::Zeroizing;

impl SshProfileRuntime {
    pub fn reserve_profile(
        &self,
        profile: &SshProfileRef,
    ) -> Result<(SshBrokerProfile, ProfileReservation)> {
        self.state.verify().inspect_err(|_| self.close_all())?;
        let store = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!(POISONED))
            .inspect_err(|_| self.close_all())?;
        let current = store.current().inspect_err(|_| self.close_all())?;
        let registration = current
            .configuration
            .profiles
            .into_iter()
            .find(|entry| &entry.profile == profile)
            .context(REGISTRY_CHANGED)?;
        let reservation = Reservations::reserve(&self.reservations, registration.clone())?;
        Ok((registration, reservation))
    }
    pub fn verify_registration(&self, registration: &SshBrokerProfile) -> Result<()> {
        self.state.verify().inspect_err(|_| self.close_all())?;
        let store = self
            .store
            .lock()
            .map_err(|_| anyhow::anyhow!(POISONED))
            .inspect_err(|_| self.close_all())?;
        let current = store.current().inspect_err(|_| self.close_all())?;
        ensure!(
            current.configuration.profiles.contains(registration),
            REGISTRY_CHANGED
        );
        Ok(())
    }
    pub async fn connect_profile(
        &self,
        profile: &SshProfileRef,
        credentials: Option<Zeroizing<Vec<u8>>>,
        deadline: Instant,
    ) -> Result<BrokerHubConnectionBinding> {
        let (_, reservation) = self.reserve_profile(profile)?;
        self.ensure_broker(&reservation, credentials, deadline)
            .await
    }
    pub async fn ensure_broker(
        &self,
        reservation: &ProfileReservation,
        credentials: Option<Zeroizing<Vec<u8>>>,
        deadline: Instant,
    ) -> Result<BrokerHubConnectionBinding> {
        ensure!(reservation.belongs_to(&self.reservations), REGISTRY_CHANGED);
        let registration = reservation.registration();
        self.verify_registration(registration)?;
        let expected = profile_binding(registration);
        if registration.launch.is_none() {
            let paths = BrokerPaths::open_existing(&registration.broker_state)?;
            let binding = BrokerClient::new(paths.management_socket())
                .prepare_hub_transport(expected, deadline)
                .await?;
            self.verify_registration(registration)?;
            paths.verify_state()?;
            return Ok(binding);
        }
        if let Ok(paths) = BrokerPaths::open_existing(&registration.broker_state) {
            let client = BrokerClient::new(paths.management_socket());
            if let Ok(status) = tokio::time::timeout_at(deadline, client.status())
                .await
                .context(LAUNCH_DEADLINE)?
            {
                ensure!(status.host_id == expected.host_id, REGISTRY_CHANGED);
                if status.config_revision == expected.config_revision && status.adopted {
                    let binding = client.prepare_hub_transport(expected, deadline).await?;
                    paths.verify_state()?;
                    self.verify_registration(registration)?;
                    return Ok(binding);
                }
            }
        }
        let launcher = self.launcher.as_ref().context(LAUNCH_UNAVAILABLE)?;
        let state = self.state.clone();
        let store = self.store.clone();
        let required = registration.clone();
        let verify = Arc::new(move || {
            state.verify()?;
            ensure!(
                store
                    .lock()
                    .map_err(|_| anyhow::anyhow!(POISONED))?
                    .resolve(&required.profile)?
                    == required,
                REGISTRY_CHANGED
            );
            Ok(())
        });
        let launch = Launch {
            executable: launcher.executable.clone(),
            owner: self.state.clone(),
            host_home: launcher.host_home.clone(),
            registration: registration.clone(),
        };
        yolo_hub_broker_launcher::ensure(launch, verify, credentials, deadline).await
    }
}

pub fn profile_binding(registration: &SshBrokerProfile) -> BrokerHubProfile {
    BrokerHubProfile {
        host_id: registration.profile.profile_id().into(),
        profile_authority_sha256: registration.profile.authority_sha256().into(),
        config_revision: registration.config_revision.clone(),
    }
}
