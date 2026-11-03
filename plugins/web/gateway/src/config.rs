use crate::{AccessToken, MAX_CONNECTIONS, MAX_ORIGINS, MAX_SESSION_LIFETIME, WebOrigin};
use anyhow::{Result, ensure};
use std::{collections::BTreeSet, net::SocketAddr, time::Duration};

pub struct GatewayConfig {
    pub(crate) bind: SocketAddr,
    pub(crate) origins: BTreeSet<WebOrigin>,
    pub(crate) token: AccessToken,
    pub(crate) lifetime: Duration,
    pub(crate) capacity: usize,
}

impl GatewayConfig {
    pub fn new(
        bind: SocketAddr,
        origins: impl IntoIterator<Item = WebOrigin>,
        token: AccessToken,
        lifetime: Duration,
        capacity: usize,
    ) -> Result<Self> {
        ensure!(
            bind.ip().is_loopback(),
            "web gateway must bind a loopback address"
        );
        let origins: BTreeSet<_> = origins.into_iter().collect();
        ensure!(
            !origins.is_empty() && origins.len() <= MAX_ORIGINS,
            "invalid web Origin allowlist"
        );
        ensure!(
            !lifetime.is_zero() && lifetime <= MAX_SESSION_LIFETIME,
            "invalid web session lifetime"
        );
        ensure!(
            capacity > 0 && capacity <= MAX_CONNECTIONS,
            "invalid web connection capacity"
        );
        Ok(Self {
            bind,
            origins,
            token,
            lifetime,
            capacity,
        })
    }
}
