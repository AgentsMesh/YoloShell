use super::*;

impl MemoryFilesystem {
    pub fn close_attempts(&self) -> usize {
        self.0.lock().unwrap().close_attempts
    }

    pub fn live_handles(&self) -> usize {
        self.0.lock().unwrap().live_handles
    }

    pub fn sftp_sessions_started(&self) -> usize {
        self.0.lock().unwrap().sftp_sessions_started
    }

    pub fn sftp_sessions_dropped(&self) -> usize {
        self.0.lock().unwrap().sftp_sessions_dropped
    }

    pub fn handles_reclaimed_on_drop(&self) -> usize {
        self.0.lock().unwrap().handles_reclaimed_on_drop
    }

    pub fn record_close_attempt(&self) {
        self.0.lock().unwrap().close_attempts += CLOSE_ATTEMPT_INCREMENT;
    }

    pub fn record_sftp_session_open(&self) {
        self.0.lock().unwrap().sftp_sessions_started += COUNTER_INCREMENT;
    }

    pub fn record_handle_open(&self) {
        self.0.lock().unwrap().live_handles += COUNTER_INCREMENT;
    }

    pub fn record_handle_close(&self) {
        let mut state = self.0.lock().unwrap();
        assert!(state.live_handles >= COUNTER_INCREMENT);
        state.live_handles -= COUNTER_INCREMENT;
    }

    pub fn record_sftp_session_drop(&self, remaining_handles: usize) {
        let mut state = self.0.lock().unwrap();
        assert!(state.live_handles >= remaining_handles);
        state.live_handles -= remaining_handles;
        state.handles_reclaimed_on_drop += remaining_handles;
        state.sftp_sessions_dropped += COUNTER_INCREMENT;
    }
}
