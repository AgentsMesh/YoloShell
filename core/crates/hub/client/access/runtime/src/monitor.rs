use crate::{ClientAccessRuntime, constants::MONITOR_INTERVAL, custody::MonitorGuard};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::watch;

impl ClientAccessRuntime {
    pub async fn run(self: Arc<Self>, mut shutdown: watch::Receiver<bool>) -> Result<()> {
        let _custody = MonitorGuard(self.registry.clone());
        let mut tick = tokio::time::interval(MONITOR_INTERVAL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if *shutdown.borrow() {
                return Ok(());
            }
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() { return Ok(()); }
                }
                _ = tick.tick() => { self.current()?; }
            }
        }
    }
}
