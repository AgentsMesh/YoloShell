use super::*;

impl MemoryFilesystem {
    pub fn set_uid_response(&self, stdout: &[u8], stderr: &[u8], status: Option<u32>) {
        self.0.lock().unwrap().uid_response = (stdout.to_vec(), stderr.to_vec(), status);
    }
    pub fn uid_response(&self) -> (Vec<u8>, Vec<u8>, Option<u32>) {
        let mut state = self.0.lock().unwrap();
        state.uid_probes += COUNTER_INCREMENT;
        state.uid_response.clone()
    }
    pub fn uid_probes(&self) -> usize {
        self.0.lock().unwrap().uid_probes
    }
    pub fn set_file_uid(&self, path: &str, uid: u32) {
        self.0.lock().unwrap().files[path].0.lock().unwrap().uid = uid;
    }
    pub fn same_file_at(&self, path: &str, expected: &MemoryFile) -> bool {
        self.0
            .lock()
            .unwrap()
            .files
            .get(path)
            .is_some_and(|actual| Arc::ptr_eq(&actual.0, &expected.0))
    }
}
