#[path = "delivery_adversarial_tests.rs"]
mod delivery_adversarial_tests;
mod delivery_durable_identity_tests;
mod delivery_stage_resources;
#[path = "../test_support/delivery_fixture.rs"]
mod delivery_fixture;
#[path = "delivery_receipt_adversarial_tests.rs"]
mod delivery_receipt_adversarial_tests;
mod delivery_receipt_lifecycle_tests;
#[path = "delivery_recovery_state_tests.rs"]
mod delivery_recovery_state_tests;
#[path = "../test_support/delivery_recovery_support.rs"]
mod delivery_recovery_support;
#[path = "delivery_recovery_tests.rs"]
mod delivery_recovery_tests;
#[path = "../test_support/delivery_system_constants.rs"]
mod delivery_system_constants;
#[path = "delivery_system_tests.rs"]
mod delivery_system_tests;
#[path = "delivery_tests.rs"]
mod delivery_tests;

use yolo_release_test_support as support;
