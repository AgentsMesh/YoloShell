use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HubPeerLifecycle {
    pub accepted: usize,
    pub closes: usize,
    pub forwarders_finished: usize,
}

impl MemoryFilesystem {
    pub fn hub_peer_lifecycles(&self) -> HashMap<russh::ChannelId, HubPeerLifecycle> {
        self.0.lock().unwrap().hub_peer_lifecycles.clone()
    }

    pub(crate) fn record_hub_peer_accepted(&self, channel: russh::ChannelId) {
        self.0
            .lock()
            .unwrap()
            .hub_peer_lifecycles
            .entry(channel)
            .or_default()
            .accepted += COUNTER_INCREMENT;
    }

    pub(crate) fn record_hub_peer_close(&self, channel: russh::ChannelId, known: bool) {
        let mut state = self.0.lock().unwrap();
        if known || state.hub_peer_lifecycles.contains_key(&channel) {
            state.hub_peer_lifecycles.entry(channel).or_default().closes += COUNTER_INCREMENT;
        }
    }

    pub(crate) fn record_hub_peer_forwarder_finished(&self, channel: russh::ChannelId) {
        self.0
            .lock()
            .unwrap()
            .hub_peer_lifecycles
            .entry(channel)
            .or_default()
            .forwarders_finished += COUNTER_INCREMENT;
    }
}
