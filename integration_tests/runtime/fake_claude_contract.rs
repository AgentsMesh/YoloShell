#![allow(dead_code, unused_imports)]

#[path = "fake_agent/claude_scenario.rs"]
mod claude_scenario;
#[path = "fake_agent/claude_wire.rs"]
mod claude_wire;
#[path = "fake_agent/claude_wire_path.rs"]
mod claude_wire_path;

mod fake_claude_authority_contract;
#[path = "fake_claude/constants.rs"]
mod fake_claude_constants;
mod fake_claude_process;
mod fake_claude_reference_endpoint;
mod fake_claude_reference_store;
mod fake_claude_reference_transition;
mod fake_claude_reference_types;
mod fake_claude_scenarios;
mod fake_claude_secure_transcript;

#[cfg(test)]
mod fake_claude_authority_tests;
#[cfg(test)]
mod fake_claude_ordering_tests;
#[cfg(test)]
mod fake_claude_process_custody_tests;
#[cfg(test)]
mod fake_claude_process_tests;
#[cfg(test)]
mod fake_claude_security_tests;
