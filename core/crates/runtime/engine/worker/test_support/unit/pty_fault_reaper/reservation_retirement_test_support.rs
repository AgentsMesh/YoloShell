use super::*;

const RETIREMENT_CHANNEL_CAPACITY: usize = 1;

pub(crate) struct ReservationRetirement {
    entered: std::sync::mpsc::SyncSender<()>,
    release: Receiver<()>,
}

pub(crate) struct ReservationRetirementProbe {
    entered: Receiver<()>,
    release: std::sync::mpsc::SyncSender<()>,
}

impl FaultReaperReservation {
    pub(crate) fn block_retirement_for_test(&mut self) -> ReservationRetirementProbe {
        let (entered_sender, entered_receiver) = sync_channel(RETIREMENT_CHANNEL_CAPACITY);
        let (release_sender, release_receiver) = sync_channel(RETIREMENT_CHANNEL_CAPACITY);
        assert!(
            self.retirement
                .replace(ReservationRetirement {
                    entered: entered_sender,
                    release: release_receiver,
                })
                .is_none()
        );
        ReservationRetirementProbe {
            entered: entered_receiver,
            release: release_sender,
        }
    }
}

impl ReservationRetirementProbe {
    pub(crate) fn wait_until_entered(&self) -> bool {
        self.entered
            .recv_timeout(HELD_REAPER_HANDOFF_TIMEOUT)
            .is_ok()
    }

    pub(crate) fn release(&self) {
        let _ = self.release.try_send(());
    }
}

impl Drop for ReservationRetirementProbe {
    fn drop(&mut self) {
        self.release();
    }
}

impl Drop for ReservationRetirement {
    fn drop(&mut self) {
        let _ = self.entered.send(());
        let _ = self.release.recv();
    }
}
