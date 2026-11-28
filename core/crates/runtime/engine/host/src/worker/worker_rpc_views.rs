impl Host {
    pub(crate) async fn runtime_view_for_manifest(
        &self,
        manifest: &TerminalManifest,
    ) -> Option<RuntimeView> {
        self.runtime_observation_for_manifest(manifest)
            .await
            .map(|observation| observation.runtime)
    }
    pub(crate) async fn runtime_observation_for_manifest(
        &self,
        manifest: &TerminalManifest,
    ) -> Option<RuntimeObservation> {
        let id = manifest.terminal_id;
        let record = self.read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(id)).ok()?;
        if validate_live_worker(&record, id, &self.boot_id).is_ok()
            && let Ok(status) = self.worker_status_if_custodied(&record).await
        {
            self.watch.agent_events(record.clone());
            return Some(RuntimeObservation::authenticated(status, &record));
        }
        Some(RuntimeObservation::unauthenticated_with_record(self.unreachable_view(&record), &record))
    }

    pub(crate) async fn view_for_manifest(&self, manifest: TerminalManifest) -> TerminalView {
        let runtime = self.runtime_observation_for_manifest(&manifest).await;
        self.view_for_manifest_runtime(manifest, runtime).await
    }

    pub(crate) async fn get_session_view_for_manifest(
        &self,
        manifest: TerminalManifest,
    ) -> TerminalView {
        let runtime = self.runtime_observation_for_manifest(&manifest).await;
        let runtime = self
            .recover_completed_runtime_for_get(&manifest, runtime)
            .await;
        self.view_for_manifest_runtime(manifest, runtime).await
    }

    pub(crate) async fn recover_completed_runtime_for_get(
        &self,
        manifest: &TerminalManifest,
        runtime: Option<RuntimeObservation>,
    ) -> Option<RuntimeObservation> {
        let Some(candidate) = runtime.as_ref() else {
            return runtime;
        };
        if manifest.archived || exact_natural_completion(&candidate.runtime).is_none() {
            return runtime;
        }
        if candidate.is_authenticated() {
            return runtime;
        }
        let Ok(record) = self.live_runtime_record(manifest.terminal_id).await else {
            return runtime;
        };
        if !runtime_view_matches(&candidate.runtime, &record)
            || self.released_natural_attachment_evidence(&record)
        {
            return runtime;
        }
        self.worker_status(&record)
            .await
            .ok()
            .map(|status| RuntimeObservation::authenticated(status, &record))
            .or(runtime)
    }

    pub(crate) async fn view_for_manifest_runtime(
        &self,
        mut manifest: TerminalManifest,
        runtime: Option<RuntimeObservation>,
    ) -> TerminalView {
        self.project_runtime_recovery_state(&mut manifest, runtime.as_ref());
        let completed_attachment = self
            .completed_attachment_view(&manifest, runtime.as_ref())
            .await;
        let (agent, authority_epoch) = self
            .binding_for_view_with_observation(&mut manifest, runtime.as_ref())
            .await;
        let observed_epoch = runtime
            .as_ref()
            .map(|observation| observation.runtime.runtime_epoch);
        let agent = runtime_epochs_pair(observed_epoch, authority_epoch)
            .then_some(agent)
            .flatten();
        let runtime = runtime.map(|observation| observation.runtime);
        TerminalView {
            manifest,
            runtime,
            agent,
            completed_attachment,
        }
    }
}
