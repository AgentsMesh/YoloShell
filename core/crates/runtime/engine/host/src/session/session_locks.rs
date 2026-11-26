#[derive(Default)]
pub(super) struct SessionLocks {
    pub(super) entries: Mutex<HashMap<Uuid, Weak<AsyncMutex<()>>>>,
}

impl SessionLocks {
    pub(super) async fn lock(&self, terminal_id: Uuid) -> OwnedMutexGuard<()> {
        let lock = {
            let mut entries = lock_recover(&self.entries);
            entries.retain(|_, entry| entry.strong_count() != 0);
            entries
                .get(&terminal_id)
                .and_then(Weak::upgrade)
                .unwrap_or_else(|| {
                    let lock = Arc::new(AsyncMutex::new(()));
                    entries.insert(terminal_id, Arc::downgrade(&lock));
                    lock
                })
        };
        lock.lock_owned().await
    }
}

pub(super) fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        tracing::error!("recovering poisoned host mutex");
        poisoned.into_inner()
    })
}
