struct CleanupTaskCustody<'a> {
    registry: &'a RemoteCleanupRegistry,
    task: Option<CleanupTask>,
}

impl CleanupTaskCustody<'_> {
    fn task(&mut self) -> &mut CleanupTask {
        let Some(task) = self.task.as_mut() else {
            unreachable!()
        };
        task
    }

    fn complete(&mut self) {
        self.task.take();
    }
}

impl Drop for CleanupTaskCustody<'_> {
    fn drop(&mut self) {
        let Some(task) = self.task.take() else {
            return;
        };
        return_tasks(self.registry, std::iter::once(task));
    }
}

fn return_tasks(registry: &RemoteCleanupRegistry, tasks: impl IntoIterator<Item = CleanupTask>) {
    registry.inner.state.lock().unwrap().tasks.extend(tasks);
    registry.inner.changed.notify_one();
}
