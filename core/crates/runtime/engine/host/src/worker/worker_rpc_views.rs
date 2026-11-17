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
        match self.live_runtime_record(manifest.terminal_id).await {
            Ok(record) => match self.worker_status_if_custodied(&record).await {
                Ok(status) => {
                    self.watch.agent_events(record.clone());
                    Some(RuntimeObservation::authenticated(status, &record))
                }
                Err(_) => Some(RuntimeObservation::unauthenticated(
                    self.unreachable_view(&record),
                )),
            },
            Err(_) => self
                .read_namespace_json::<RuntimeRecord>(
                    &self.paths.runtime_record(manifest.terminal_id),
                )
                .ok()
                .map(|record| RuntimeObservation::unauthenticated(self.unreachable_view(&record))),
        }
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
        let Ok(record) = self.live_runtime_record(manifest.terminal_id).await else {
            return runtime;
        };
        if !runtime_view_matches(&candidate.runtime, &record)
            || self.released_natural_attachment_evidence(&record)
            || candidate.is_authenticated()
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
        let (agent, authority_epoch) = self.binding_for_view(&mut manifest).await;
        // The runtime observation and the binding authority are separate
        // reads. Reject the pairing if a concurrent lifecycle change replaced
        // the runtime between them. This keeps GET/LIST lock-free and bounded
        // by their callers' deadlines.
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
