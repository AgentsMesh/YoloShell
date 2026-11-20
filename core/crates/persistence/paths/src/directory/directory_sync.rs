use super::*;

#[cfg(any(test, feature = "persistence-test-support"))]
const INJECTED_SYNC_FAILURE: &str = "injected private directory sync failure";

impl PrivateDirectory {
    pub fn sync(&self) -> Result<()> {
        #[cfg(any(test, feature = "persistence-test-support"))]
        if FAIL_NEXT_SYNC.with(|flag| flag.replace(false)) {
            anyhow::bail!(INJECTED_SYNC_FAILURE);
        }
        self.file.sync_all().context("sync private directory")
    }
}

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static FAIL_NEXT_SYNC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn fail_next() {
    FAIL_NEXT_SYNC.with(|flag| flag.set(true));
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn clear_failure() {
    FAIL_NEXT_SYNC.with(|flag| flag.set(false));
}
