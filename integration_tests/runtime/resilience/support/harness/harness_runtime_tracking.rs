use super::*;

const TRACKED_RECORD_BIND_CONTEXT: &str = "bind tracked runtime record";
const TRACKED_TREE_CAPTURE_CONTEXT: &str = "capture tracked runtime process tree";

impl Harness {
    pub(super) fn bind_cleanup_records(
        &self,
        records: &[RuntimeRecord],
    ) -> Vec<runtime_record_authority::BoundRuntimeRecord> {
        let tracked = self
            .runtimes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        records
            .iter()
            .map(|record| {
                tracked
                    .iter()
                    .find_map(|bound| bound.refresh(record).ok())
                    .expect(TRACKED_RECORD_BIND_CONTEXT)
            })
            .collect()
    }

    pub(crate) fn track_view(&self, view: &yolo_runtime::TerminalView) {
        let paths = self.paths();
        let terminal_id = view.manifest.terminal_id;
        let record: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
        let mut tracked = self
            .runtimes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let existing = tracked.iter().position(|bound| {
            bound.record().terminal_id == record.terminal_id
                && bound.record().runtime_epoch == record.runtime_epoch
        });
        let bound = match existing {
            Some(index) => {
                let bound = tracked[index]
                    .refresh_view(&record, view)
                    .expect(TRACKED_RECORD_BIND_CONTEXT);
                tracked[index] = bound.clone();
                bound
            }
            None => {
                let bound =
                    runtime_record_authority::BoundRuntimeRecord::bind_view(&paths, record, view)
                        .expect(TRACKED_RECORD_BIND_CONTEXT);
                tracked.push(bound.clone());
                bound
            }
        };
        drop(tracked);
        self.cleanup_trees
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .ensure_cleanup_records(&paths, std::slice::from_ref(&bound))
            .expect(TRACKED_TREE_CAPTURE_CONTEXT);
    }

    pub(crate) async fn resume_and_track(
        &self,
        terminal_id: Uuid,
        force: bool,
    ) -> std::result::Result<yolo_runtime::TerminalView, ClientError> {
        let view = self.client().resume(terminal_id, force).await?;
        self.track_view(&view);
        Ok(view)
    }

    pub(crate) async fn resume_with_preconditions_and_track(
        &self,
        selection: yolo_runtime::model::TerminalLifecycleSelection,
        force: bool,
    ) -> std::result::Result<yolo_runtime::TerminalView, ClientError> {
        let view = self
            .client()
            .resume_with_preconditions(selection, force)
            .await?;
        self.track_view(&view);
        Ok(view)
    }
}
