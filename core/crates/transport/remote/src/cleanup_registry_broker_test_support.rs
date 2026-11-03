impl RemoteCleanupRegistry {
    #[doc(hidden)]
    pub fn reserve_for_broker_test(&self) -> Result<RemoteCleanupReservation, RemoteError> {
        self.reserve()
    }
}

impl RemoteCleanupReservation {
    #[doc(hidden)]
    pub fn launch_for_broker_test<F>(self, future: F)
    where
        F: Future<Output = CleanupResult> + Send + 'static,
    {
        self.launch(future);
    }
}
