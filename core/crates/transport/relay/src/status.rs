use super::RelayStatus;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub(crate) struct RelayStatusSink(mpsc::UnboundedSender<RelayStatus>);

pub(crate) trait RelayStatusTarget: Send + Sync {
    fn send_status(&self, status: RelayStatus);
}

impl RelayStatusSink {
    pub(crate) fn send(&self, status: RelayStatus) {
        let _ = self.0.send(status);
    }
}

impl RelayStatusTarget for RelayStatusSink {
    fn send_status(&self, status: RelayStatus) {
        self.send(status);
    }
}

impl RelayStatusTarget for mpsc::UnboundedSender<RelayStatus> {
    fn send_status(&self, status: RelayStatus) {
        let _ = self.send(status);
    }
}

impl From<mpsc::UnboundedSender<RelayStatus>> for RelayStatusSink {
    fn from(sender: mpsc::UnboundedSender<RelayStatus>) -> Self {
        Self(sender)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/status_tests.rs"]
mod tests;
