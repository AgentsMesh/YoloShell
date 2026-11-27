use super::*;
use crate::SessionGenerationPolicy;
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use yolo_agent_contract::{AgentEvidence, ObservedProcess, ResumePlan};

pub(super) struct EvidenceAdapter {
    evidence: Option<AgentEvidence>,
    fails: bool,
    owns: bool,
    inspections: Arc<AtomicUsize>,
    policy: SessionGenerationPolicy,
}

impl AgentAdapter for EvidenceAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }
    fn matches_process(&self, _: &ObservedProcess) -> bool {
        false
    }
    fn owns_evidence_path(&self, _: &Path, _: &DetectionContext) -> bool {
        self.owns
    }
    fn session_generation_policy(&self) -> SessionGenerationPolicy {
        self.policy
    }
    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }
    fn inspect_evidence(&self, _: &Path, _: &Path) -> Result<Option<AgentEvidence>> {
        self.inspections.fetch_add(1, Ordering::SeqCst);
        if self.fails {
            Err(anyhow!("injected evidence failure"))
        } else {
            Ok(self.evidence.clone())
        }
    }
    fn resume_plan(&self, _: &AgentBinding) -> ResumePlan {
        ResumePlan {
            executable: String::new(),
            args: Vec::new(),
            environment: Default::default(),
        }
    }
    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

fn validates(binding: &AgentBinding, context: &DetectionContext, evidence: AgentEvidence) -> bool {
    validate_evidence_binding(
        &EvidenceAdapter {
            evidence: Some(evidence),
            fails: false,
            owns: true,
            inspections: Arc::new(AtomicUsize::new(0)),
            policy: SessionGenerationPolicy::AuthoritativeOnly,
        },
        binding,
        context,
    )
    .is_ok()
}

#[test]
fn evidence_validation_distinguishes_read_missing_and_every_identity_mismatch() {
    let (_directory, context, binding, _root) = fixture();
    let evidence = AgentEvidence {
        provider_session_id: binding.provider_session_id.clone(),
        cwd: PathBuf::from(&binding.cwd),
        path: PathBuf::from(&binding.evidence_path),
        data_home: PathBuf::from(&binding.data_home),
        state: AgentState::Running,
    };
    assert!(validates(&binding, &context, evidence.clone()));
    assert!(
        validate_evidence_binding(
            &EvidenceAdapter {
                evidence: None,
                fails: false,
                owns: true,
                inspections: Arc::new(AtomicUsize::new(0)),
                policy: SessionGenerationPolicy::AuthoritativeOnly,
            },
            &binding,
            &context,
        )
        .is_err()
    );
    assert!(
        validate_evidence_binding(
            &EvidenceAdapter {
                evidence: Some(evidence.clone()),
                fails: true,
                owns: true,
                inspections: Arc::new(AtomicUsize::new(0)),
                policy: SessionGenerationPolicy::AuthoritativeOnly,
            },
            &binding,
            &context,
        )
        .is_err()
    );
    let mut mismatched = evidence.clone();
    mismatched.provider_session_id.push_str("-other");
    assert!(!validates(&binding, &context, mismatched));
    let mut mismatched = evidence.clone();
    mismatched.path.push("other");
    assert!(!validates(&binding, &context, mismatched));
    let mut mismatched = evidence.clone();
    mismatched.cwd.push("other");
    assert!(!validates(&binding, &context, mismatched));
    let mut mismatched = evidence;
    mismatched.data_home.push("other");
    assert!(!validates(&binding, &context, mismatched));
}

#[test]
fn foreign_evidence_is_rejected_before_adapter_inspection() {
    let (_directory, context, binding, _root) = fixture();
    let inspections = Arc::new(AtomicUsize::new(0));
    let adapter = EvidenceAdapter {
        evidence: None,
        fails: true,
        owns: false,
        inspections: Arc::clone(&inspections),
        policy: SessionGenerationPolicy::AuthoritativeOnly,
    };
    let error = validate_evidence_binding(&adapter, &binding, &context).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(crate::constants::message::EVIDENCE_PATH)
    );
    assert_eq!(inspections.load(Ordering::SeqCst), 0);
}

pub(super) fn hook_required_adapter() -> EvidenceAdapter {
    EvidenceAdapter {
        evidence: None,
        fails: false,
        owns: false,
        inspections: Arc::new(AtomicUsize::new(0)),
        policy: SessionGenerationPolicy::HookRequired,
    }
}
