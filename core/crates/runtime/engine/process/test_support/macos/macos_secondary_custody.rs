impl MacosSecondaryCustody {
    pub fn pty_slave_path_for_test(&self) -> &std::path::Path {
        self.pty_master.slave_path()
    }
}
