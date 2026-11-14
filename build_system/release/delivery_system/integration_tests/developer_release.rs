#[path = "../test_support/delivery_fixture.rs"]
mod delivery_fixture;
#[path = "../test_support/delivery_recovery_support.rs"]
mod delivery_recovery_support;
#[path = "developer_release_acceptance_adversarial_tests.rs"]
mod developer_release_acceptance_adversarial_tests;
#[path = "../test_support/developer_release_acceptance_support.rs"]
mod developer_release_acceptance_support;
#[path = "developer_release_adversarial_tests.rs"]
mod developer_release_adversarial_tests;
#[path = "developer_release_command_tests.rs"]
mod developer_release_command_tests;
mod developer_release_executable_custody_tests;
#[path = "../test_support/developer_release_fixture.rs"]
mod developer_release_fixture;
#[path = "../test_support/developer_release_fixture_archive.rs"]
mod developer_release_fixture_archive;
#[path = "../test_support/developer_release_fixture_runner.rs"]
mod developer_release_fixture_runner;
#[path = "developer_release_inventory_tests.rs"]
mod developer_release_inventory_tests;
#[path = "developer_release_replacement_runner.rs"]
mod developer_release_replacement_runner;
#[path = "developer_release_tests.rs"]
mod developer_release_tests;
#[path = "../test_support/verify_acceptance_support.rs"]
mod verify_acceptance_support;
#[path = "verify_acceptance_tests.rs"]
mod verify_acceptance_tests;

use yolo_release_test_support as support;
