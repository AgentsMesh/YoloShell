use super::protocol::constants::{ACTIVATED_DECISION, RELEASE_DECISION, WORKER_READY_DECISION_TAG};
use super::*;

const ACTIVATION_GENERATION: u64 = 19;
const STALE_ACTIVATION_GENERATION: u64 = ACTIVATION_GENERATION + 1;

#[test]
fn worker_activation_consumes_only_for_its_exact_offer_and_generation() {
    let exact_offer = offer();
    CustodyWorkerActivated::new(&exact_offer, ACTIVATION_GENERATION)
        .unwrap()
        .into_exact(&exact_offer, ACTIVATION_GENERATION)
        .unwrap();

    assert!(
        CustodyWorkerActivated::new(&exact_offer, ACTIVATION_GENERATION)
            .unwrap()
            .into_exact(&offer(), ACTIVATION_GENERATION)
            .is_err()
    );
    assert!(
        CustodyWorkerActivated::new(&exact_offer, ACTIVATION_GENERATION)
            .unwrap()
            .into_exact(&exact_offer, STALE_ACTIVATION_GENERATION)
            .is_err()
    );
}

#[test]
fn worker_activation_rejects_the_default_generation() {
    assert!(CustodyWorkerActivated::new(&offer(), u64::default()).is_err());
}

#[test]
fn ready_disposition_round_trips_activation_and_exact_release_without_ambiguity() {
    let exact_offer = offer();
    let dispositions = [
        CustodyWorkerReadyDisposition::activated(
            CustodyWorkerActivated::new(&exact_offer, ACTIVATION_GENERATION).unwrap(),
        ),
        CustodyWorkerReadyDisposition::release(CustodyControl::release(&exact_offer).unwrap()),
    ];

    for disposition in dispositions {
        let encoded = serde_json::to_vec(&disposition).unwrap();
        let decoded: CustodyWorkerReadyDisposition = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, disposition);
    }
}

#[test]
fn ready_disposition_requires_an_explicit_decision_tag() {
    let exact_offer = offer();
    let activated = CustodyWorkerReadyDisposition::activated(
        CustodyWorkerActivated::new(&exact_offer, ACTIVATION_GENERATION).unwrap(),
    );
    let release =
        CustodyWorkerReadyDisposition::release(CustodyControl::release(&exact_offer).unwrap());

    assert_eq!(decision(&activated), ACTIVATED_DECISION);
    assert_eq!(decision(&release), RELEASE_DECISION);
    assert!(
        serde_json::from_value::<CustodyWorkerReadyDisposition>(
            serde_json::to_value(
                CustodyWorkerActivated::new(&exact_offer, ACTIVATION_GENERATION).unwrap(),
            )
            .unwrap(),
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<CustodyWorkerReadyDisposition>(
            serde_json::to_value(CustodyControl::release(&exact_offer).unwrap()).unwrap(),
        )
        .is_err()
    );
}

fn decision(disposition: &CustodyWorkerReadyDisposition) -> String {
    serde_json::to_value(disposition).unwrap()[WORKER_READY_DECISION_TAG]
        .as_str()
        .unwrap()
        .to_owned()
}

fn offer() -> CustodyOffer {
    CustodyOffer::new(
        &super::tests::request(),
        super::tests::descriptor_set(),
        super::tests::digest(),
    )
}
