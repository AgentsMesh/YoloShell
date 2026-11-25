#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CreationFailureResolution {
    Active,
    Resolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CreationRuntimeEvidence {
    Missing,
    MatchingPending,
    Ready,
    Failed(CreationFailureResolution),
    Unknown,
    Mismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::host) enum CreationEvidenceIdentity {
    Missing,
    Current,
    Previous,
    Invalid,
}

impl Host {
    pub(crate) fn creation_runtime_evidence(
        &self,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
    ) -> CreationRuntimeEvidence {
        let terminal_id = binding.result.terminal_id;
        let launch = read_creation_evidence::<WorkerLaunchSpec>(
            &self.paths.worker_launch(terminal_id),
            "creation worker launch",
        );
        let runtime = read_creation_evidence::<RuntimeRecord>(
            &self.paths.runtime_record(terminal_id),
            "creation runtime record",
        );
        let (launch, runtime) = match (launch, runtime) {
            (Ok(launch), Ok(runtime)) => (launch, runtime),
            (launch, runtime) => {
                let launch_error = launch.as_ref().err().map(ToString::to_string);
                let runtime_error = runtime.as_ref().err().map(ToString::to_string);
                tracing::error!(
                    %terminal_id,
                    ?launch_error,
                    ?runtime_error,
                    "creation evidence is unreadable"
                );
                return CreationRuntimeEvidence::Unknown;
            }
        };
        let launch_identity =
            classify_creation_launch(launch.as_ref(), binding, planned, &self.paths);
        if launch_identity == CreationEvidenceIdentity::Invalid {
            return CreationRuntimeEvidence::Mismatch;
        }
        let current_launch = match (launch_identity, launch.as_ref()) {
            (CreationEvidenceIdentity::Current, Some(value)) => Some(value),
            _ => None,
        };
        let runtime_identity = classify_creation_runtime(
            runtime.as_ref(),
            current_launch,
            binding,
            planned,
            &self.paths,
        );
        let record = match (launch_identity, runtime_identity, runtime.as_ref()) {
            (_, CreationEvidenceIdentity::Invalid, _)
            | (CreationEvidenceIdentity::Previous, CreationEvidenceIdentity::Current, _) => {
                return CreationRuntimeEvidence::Mismatch;
            }
            (_, CreationEvidenceIdentity::Current, Some(record)) => record,
            (CreationEvidenceIdentity::Current, _, _) => {
                return CreationRuntimeEvidence::Unknown;
            }
            _ => return CreationRuntimeEvidence::Missing,
        };
        if record.controller_generation > self.controller_generation {
            return CreationRuntimeEvidence::Mismatch;
        }
        match record.bootstrap_delivery {
            BootstrapDelivery::NotRequired
            | BootstrapDelivery::Survivor
            | BootstrapDelivery::Delivered => CreationRuntimeEvidence::Ready,
            BootstrapDelivery::Pending
                if self.creation_failure_resolution(record)
                    == CreationFailureResolution::Resolved =>
            {
                CreationRuntimeEvidence::Failed(CreationFailureResolution::Resolved)
            }
            BootstrapDelivery::Pending => CreationRuntimeEvidence::MatchingPending,
            BootstrapDelivery::Failed => {
                CreationRuntimeEvidence::Failed(self.creation_failure_resolution(&record))
            }
            BootstrapDelivery::Unknown => CreationRuntimeEvidence::Unknown,
        }
    }

    pub(crate) async fn authenticated_creation_runtime_evidence(
        &self,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
    ) -> CreationRuntimeEvidence {
        let evidence = self.creation_runtime_evidence(binding, planned);
        if evidence != CreationRuntimeEvidence::Ready {
            return evidence;
        }
        let terminal_id = binding.result.terminal_id;
        let Ok(record) =
            self.read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(terminal_id))
        else {
            return CreationRuntimeEvidence::Unknown;
        };
        let Ok(authenticated) =
            crate::worker::authenticated_runtime(&record, self.controller_generation).await
        else {
            return CreationRuntimeEvidence::Unknown;
        };
        let confirmed = self.creation_runtime_evidence(binding, planned);
        if confirmed != CreationRuntimeEvidence::Ready {
            return confirmed;
        }
        let Ok(current) =
            self.read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(terminal_id))
        else {
            return CreationRuntimeEvidence::Unknown;
        };
        if authenticated != current {
            return CreationRuntimeEvidence::Mismatch;
        }
        CreationRuntimeEvidence::Ready
    }

    pub(crate) fn creation_failure_resolution(
        &self,
        record: &RuntimeRecord,
    ) -> CreationFailureResolution {
        let previous_boot = process::boot_identity_relation(&record.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different;
        if previous_boot
            || (process_identity_is_gone(record.worker_pid, record.worker_start_time)
                && process_identity_is_gone(record.child_pid, record.child_start_time))
        {
            CreationFailureResolution::Resolved
        } else {
            CreationFailureResolution::Active
        }
    }
}

include!("creation_runtime_identity.rs");

pub(crate) fn creation_runtime_mismatch() -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::IDENTITY_MISMATCH,
        CREATION_RUNTIME_MISMATCH,
        false,
    )
}

pub(in crate::host) fn creation_launch_uncertain() -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::DELIVERY_UNKNOWN,
        CREATION_LAUNCH_UNCERTAIN,
        true,
    )
}

pub(crate) fn creation_launch_failed() -> ErrorBody {
    ErrorBody::new(
        crate::protocol::error_code::WORKER_START_FAILED,
        CREATION_LAUNCH_FAILED,
        true,
    )
}
