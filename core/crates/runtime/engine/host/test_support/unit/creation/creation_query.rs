use super::*;
#[path = "creation_query_snapshot.rs"]
mod snapshots;

pub(super) struct QueryFixture {
    pub host: Host,
    pub request: CreateTerminal,
    pub binding: CreationBinding,
    pub ledger_path: PathBuf,
    pub directory: HostTestDirectory,
}

impl QueryFixture {
    pub fn snapshot(&self) -> impl std::fmt::Debug + Eq {
        snapshots::snapshot(self.directory.path())
    }

    pub fn new() -> Self {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(
            directory.path().join(QUERY_HOST_HOME_DIRECTORY),
        ))
        .unwrap();
        let (request, binding, ledger_path) =
            reserve_test_creation(&host, directory.path(), QUERY_NAME);
        Self {
            directory: HostTestDirectory::capture(directory, &host.paths),
            host,
            request,
            binding,
            ledger_path,
        }
    }

    pub fn phase(&mut self, phase: CreationPhase) {
        let mut ledger = lock_recover(&self.host.creation_capabilities);
        let mut candidate = ledger.clone();
        let binding = candidate
            .bindings
            .iter_mut()
            .find(|entry| entry.result == self.binding.result)
            .unwrap();
        binding.phase = phase;
        if phase == CreationPhase::Reserved {
            binding.launch = None;
            binding.launch_generation = RESERVED_CREATION_LAUNCH_GENERATION;
        } else {
            binding.launch = Some(CreationLaunchIdentity::new());
            binding.launch_generation = INITIAL_CREATION_LAUNCH_GENERATION;
        }
        self.binding = binding.clone();
        ledger.install(&self.ledger_path, candidate).unwrap();
    }

    pub fn reserve(&self, request: &CreateTerminal) -> CreationBinding {
        let fingerprint = CreationSpec::fingerprint(request).unwrap();
        let spec = CreationSpec::resolve(request, fingerprint).unwrap();
        lock_recover(&self.host.creation_capabilities)
            .reserve(
                &self.ledger_path,
                request.creation_token,
                spec,
                &self.host.creation_receipts,
            )
            .unwrap()
    }

    pub fn receipt(&self, completion: CreationCompletion) -> CreationReceipt {
        CreationReceipt::from_binding(&self.binding, completion)
    }

    pub fn publish(&self, receipt: &CreationReceipt, token_index: bool, session_index: bool) {
        let store = &self.host.creation_receipts;
        if token_index {
            publish_receipt_at(
                &store.staging,
                &store.token_index,
                &receipt.creation_token_digest,
                receipt,
            )
            .unwrap();
        }
        if session_index {
            publish_receipt_at(
                &store.staging,
                &store.session_index,
                &receipt.result.terminal_id.hyphenated().to_string(),
                receipt,
            )
            .unwrap();
        }
    }

    pub fn remove_binding(&self) {
        lock_recover(&self.host.creation_capabilities)
            .remove_completed_binding(&self.ledger_path, self.binding.result)
            .unwrap();
    }

    pub async fn observe(
        &self,
        query: &CreateTerminal,
    ) -> std::result::Result<CreationQueryResult, ErrorBody> {
        let before = snapshots::snapshot(self.directory.path());
        let response = self
            .host
            .dispatch(&request(method::QUERY_CREATION, query))
            .await;
        assert_eq!(snapshots::snapshot(self.directory.path()), before);
        if !response.ok {
            return Err(response.error.unwrap());
        }
        let result: CreationQueryResult = response.decode_payload().unwrap();
        assert!(result.matches_request(query.creation_token));
        Ok(result)
    }

    pub async fn outcome(&self) -> CreationQueryOutcome {
        self.observe(&self.request).await.unwrap().outcome
    }

    pub fn finalize_request(&self) -> FinalizeCreationRequest {
        FinalizeCreationRequest {
            creation_token: self.request.creation_token,
            terminal_id: self.binding.result.terminal_id,
            creation_capability: self.binding.result.creation_capability,
        }
    }
}
