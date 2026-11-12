use super::pty_master_closure_policy_fixture::{
    ACK_SOURCE, RUNTIME_SOURCE, SETTLEMENT_SOURCE, STARTUP_CUSTODY_RELEASE_SOURCE,
    STARTUP_CUSTODY_SOURCE, contract_fixture, messages,
};
use crate::pty_master_closure_contract as contract;

const FUTURE_PATH: &str = "core/crates/runtime/engine/worker/src/future_master_closure.rs";
const FUTURE_TEST_PATH: &str =
    "core/crates/runtime/engine/worker/src/future_master_closure_tests.rs";
const POLICY_TEXT: &str = "restricted to the typed settlement routes";
const CONTRACT_TEXT: &str = "definition or consumption contract changed";

#[test]
fn accepts_the_two_exact_linear_settlement_routes() {
    let fixture = contract_fixture();
    let actual = messages(&fixture);
    assert!(actual.is_empty(), "{actual:#?}");
}

#[test]
fn rejects_future_and_duplicate_capability_consumers() {
    let future = contract_fixture();
    future.write(
        FUTURE_PATH,
        "fn bypass(authority: Authority) { authority.take_local_aliases_closed(); }\n",
    );
    assert!(
        messages(&future)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );

    let duplicate = contract_fixture();
    duplicate.write(
        contract::RUNTIME_RELEASE_PATH,
        RUNTIME_SOURCE.replace(
            "consume(proof);",
            "consume(proof);\nlet duplicate = local.finalize_host_release(acknowledgement).unwrap().mint_final_proof();",
        ),
    );
    let result = messages(&duplicate);
    assert!(result.iter().any(|item| item.contains(POLICY_TEXT)));
    assert!(result.iter().any(|item| item.contains(CONTRACT_TEXT)));
}

#[test]
fn rejects_swapped_host_and_no_secondary_routes() {
    let host_bypass = contract_fixture();
    host_bypass.write(
        contract::RUNTIME_RELEASE_PATH,
        RUNTIME_SOURCE.replace(
            "finalize_host_release(acknowledgement)",
            "finalize_without_secondary_custody()",
        ),
    );
    assert!(
        messages(&host_bypass)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );

    let startup_bypass = contract_fixture();
    startup_bypass.write(
        contract::STARTUP_CUSTODY_RELEASE_PATH,
        STARTUP_CUSTODY_RELEASE_SOURCE.replace(
            "finalize_without_secondary_custody()",
            "finalize_host_release(forged)",
        ),
    );
    assert!(
        messages(&startup_bypass)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );
}

#[test]
fn rejects_aliased_or_external_host_acknowledgement_calls() {
    let aliased = contract_fixture();
    aliased.write(
        contract::CUSTODY_RELEASE_PATH,
        ACK_SOURCE.replace(
            "let acknowledgement = self.verifier.acknowledge(exact);",
            "let verifier = self.verifier;\nlet acknowledgement = verifier.acknowledge(exact);",
        ),
    );
    assert!(
        messages(&aliased)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );

    let external = contract_fixture();
    external.write(
        FUTURE_PATH,
        "fn forge(verifier: PtyHostReleaseVerifier, exact: ExactCustodyReleasedAck) { verifier.acknowledge(exact); }\n",
    );
    assert!(
        messages(&external)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );
}

#[test]
fn startup_acknowledgements_require_the_exact_verifier_and_proof() {
    let wrong_verifier = contract_fixture();
    wrong_verifier.write(
        contract::STARTUP_CUSTODY_PATH,
        STARTUP_CUSTODY_SOURCE.replace(
            "verifier.clone().acknowledge(exact)",
            "other.clone().acknowledge(exact)",
        ),
    );
    assert!(
        messages(&wrong_verifier)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );

    let forged_gone = contract_fixture();
    forged_gone.write(
        contract::STARTUP_CUSTODY_RELEASE_PATH,
        STARTUP_CUSTODY_RELEASE_SOURCE.replace(
            "acknowledge_holder_gone(gone)",
            "acknowledge_holder_gone(forged)",
        ),
    );
    assert!(
        messages(&forged_gone)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );
}

#[test]
fn rejects_direct_or_self_final_proof_construction() {
    let direct = contract_fixture();
    direct.write(
        FUTURE_PATH,
        "fn forge() -> PtyMasterAliasesClosed { PtyMasterAliasesClosed {} }\n",
    );
    assert!(
        messages(&direct)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );

    let self_mint = contract_fixture();
    self_mint.write(
        FUTURE_PATH,
        "impl PtyMasterAliasesClosed { pub(crate) fn forge() -> Self { Self {} } }\n",
    );
    assert!(
        messages(&self_mint)
            .iter()
            .any(|item| item.contains(POLICY_TEXT))
    );
}

#[test]
fn rejects_crate_private_or_missing_mint_definition() {
    let restricted = contract_fixture();
    restricted.write(
        contract::SETTLEMENT_PATH,
        SETTLEMENT_SOURCE.replace("pub fn mint_final_proof", "pub(crate) fn mint_final_proof"),
    );
    let result = messages(&restricted);
    assert!(result.iter().any(|item| item.contains(POLICY_TEXT)));
    assert!(result.iter().any(|item| item.contains(CONTRACT_TEXT)));
}

#[test]
fn ignores_test_only_capability_exercises() {
    let fixture = contract_fixture();
    fixture.write(
        FUTURE_PATH,
        "#[cfg(test)] fn bypass(authority: Authority) { authority.take_local_aliases_closed(); }\n",
    );
    fixture.write(
        FUTURE_TEST_PATH,
        "fn bypass(authority: Authority) { authority.take_local_aliases_closed(); }\n",
    );
    assert!(messages(&fixture).is_empty());
}
