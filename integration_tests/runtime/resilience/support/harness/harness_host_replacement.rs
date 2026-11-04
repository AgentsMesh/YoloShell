use super::host_adoption::wait_for_runtime_adoptions;
use super::*;

const REPLACEMENT_LOG_IDENTITY_DETAIL: &str = "replacement Host stderr log identity";

impl Harness {
    pub(crate) async fn replace_host(&mut self, child: SpawnedHost) {
        let expected = self
            .runtime_adoption_expectations()
            .unwrap_or_else(|error| panic!("{error}"));
        self.host
            .require_same_log(&child)
            .expect(REPLACEMENT_LOG_IDENTITY_DETAIL);
        self.host = HostGuard::new(child);
        let status = wait_for_host_status(self.home.path(), &mut self.host).await;
        wait_for_runtime_adoptions(self.home.path(), &mut self.host, status, expected)
            .await
            .unwrap_or_else(|error| panic!("{error}"));
    }
}
