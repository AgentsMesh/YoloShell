use crate::record::{AcceptanceCandidateBinding, AcceptancePlatform, AcceptanceProviderVersion};
use crate::record_validation::{validate_binding_text, validate_providers};
use anyhow::Result;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptanceExpectation {
    pub candidate: AcceptanceCandidateBinding,
    pub checklist_schema: String,
    pub providers: Vec<AcceptanceProviderVersion>,
    pub desktop_os: AcceptancePlatform,
    pub remote_os: AcceptancePlatform,
    pub operator_identity: String,
    pub now_unix_seconds: u64,
}

impl AcceptanceExpectation {
    pub fn new(
        candidate: AcceptanceCandidateBinding,
        checklist_schema: impl Into<String>,
        providers: Vec<AcceptanceProviderVersion>,
        desktop_os: AcceptancePlatform,
        remote_os: AcceptancePlatform,
        operator_identity: impl Into<String>,
        now_unix_seconds: u64,
    ) -> Result<Self> {
        let value = Self {
            candidate,
            checklist_schema: checklist_schema.into(),
            providers,
            desktop_os,
            remote_os,
            operator_identity: operator_identity.into(),
            now_unix_seconds,
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.candidate.validate()?;
        validate_binding_text(&self.checklist_schema)?;
        validate_providers(&self.providers)?;
        self.desktop_os.validate()?;
        self.remote_os.validate()?;
        validate_binding_text(&self.operator_identity)
    }
}
