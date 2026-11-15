#[path = "../test_support/constants.rs"]
mod constants;
mod coverage_branch_tests;
mod edge_case_tests;
mod glob_coverage_tests;
mod layout_tests;
mod parser_recovery_tests;
mod path_policy_tests;
mod postfix_ownership_tests;
mod production_test_boundary_tests;
mod reachability_cfg_tests;
mod reachability_tests;
mod repository_activation_tests;
mod source_budget_tests;
mod starlark_adversarial_tests;
mod starlark_resolution_tests;
#[path = "../test_support/support.rs"]
mod support;
mod target_ownership_tests;
mod test_budget_tests;
mod test_ownership_tests;
