use super::*;

impl MemoryFilesystem {
    pub fn live_commands(&self) -> usize {
        self.0.lock().unwrap().commands.live
    }

    pub fn command_signals(&self) -> usize {
        self.0.lock().unwrap().commands.signals
    }

    pub fn command_closes(&self) -> usize {
        self.0.lock().unwrap().commands.closes
    }

    pub fn command_outputs_started(&self) -> usize {
        self.0.lock().unwrap().commands.output_started
    }

    pub fn record_live_command(&self) {
        self.0.lock().unwrap().commands.live += COUNTER_INCREMENT;
    }

    pub fn record_command_output(&self) {
        self.0.lock().unwrap().commands.output_started += COUNTER_INCREMENT;
    }

    pub fn record_command_signal(&self, signal: &russh::Sig) {
        if !matches!(signal, russh::Sig::KILL) {
            return;
        }
        let mut state = self.0.lock().unwrap();
        state.commands.signals += COUNTER_INCREMENT;
        state.commands.live = state.commands.live.saturating_sub(COUNTER_INCREMENT);
    }

    pub fn record_command_close(&self) {
        self.0.lock().unwrap().commands.closes += COUNTER_INCREMENT;
    }
}
