#[path = "../linux_containment_kernel_tests/broker.rs"]
mod broker;
#[path = "../linux_containment_kernel_tests/custody.rs"]
mod custody;
#[path = "../linux_containment_kernel_tests/custody_fixture.rs"]
mod custody_fixture;
#[path = "../linux_containment_kernel_tests/retirement.rs"]
mod custody_retirement;
#[path = "../../test_support/linux/linux_custody_retirement.rs"]
mod custody_retirement_fixture;
#[path = "../linux_containment_kernel_tests/custody_substitution.rs"]
mod custody_substitution;
#[path = "../linux_containment_kernel_tests/devpts.rs"]
mod devpts;
#[path = "../linux_containment_kernel_tests/devpts_assertions.rs"]
mod devpts_assertions;
#[path = "../linux_containment_kernel_tests/devpts_fixture.rs"]
mod devpts_fixture;
#[path = "../linux_containment_kernel_tests/guardian.rs"]
mod guardian;
#[path = "../linux_containment_kernel_tests/guardian_child.rs"]
mod guardian_child;
#[path = "../linux_containment_kernel_tests/guardian_fixture.rs"]
mod guardian_fixture;
#[path = "../linux_containment_kernel_tests/guardian_monitor.rs"]
mod guardian_monitor;
#[path = "../linux_containment_kernel_tests/guardian_process.rs"]
mod guardian_process;
#[path = "../linux_containment_kernel_tests/guardian_race.rs"]
mod guardian_race;
#[path = "../linux_containment_kernel_tests/record.rs"]
mod record;
#[path = "../linux_containment_kernel_tests/recovery.rs"]
mod recovery;
#[path = "../linux_containment_kernel_tests/retirement.rs"]
mod retirement;
#[path = "../linux_containment_kernel_tests/custody.rs"]
mod secondary_custody;
#[path = "../linux_containment_kernel_tests/secondary_custody_fixture.rs"]
mod secondary_custody_fixture;
#[path = "../linux_containment_kernel_tests/settlement.rs"]
mod settlement;
#[path = "../linux_containment_kernel_tests/settlement_witness.rs"]
mod settlement_witness;
#[path = "../linux_containment_kernel_tests/settlement_witness_adversarial.rs"]
mod settlement_witness_adversarial;
#[path = "../../test_support/linux/linux_settlement_witness.rs"]
mod settlement_witness_fixture;
#[path = "../linux_containment_kernel_tests/settlement_witness_resources.rs"]
mod settlement_witness_resources;
#[path = "../linux_containment_kernel_tests/support.rs"]
mod support;

#[path = "../linux_containment_kernel_tests/custody_retirement.rs"]
mod custody_retirement_exact;
#[path = "../linux_containment_kernel_tests/secondary_custody.rs"]
mod secondary_custody_exact;
#[path = "../linux_containment_kernel_tests/secondary_custody_fixture.rs"]
mod secondary_custody_exact_fixture;
