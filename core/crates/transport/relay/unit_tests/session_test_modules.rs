#[cfg(test)]
#[path = "session_basic.rs"]
mod basic_tests;
#[cfg(test)]
#[path = "session_cancellation.rs"]
mod cancellation_tests;
#[cfg(test)]
#[path = "session_completed.rs"]
mod completed_tests;
#[cfg(test)]
#[path = "session_completed_adversarial.rs"]
mod completed_adversarial_tests;
#[cfg(test)]
#[path = "session_completed_loss.rs"]
mod completed_loss_tests;
#[cfg(test)]
#[path = "session_control_intent.rs"]
mod control_intent_tests;
#[cfg(test)]
#[path = "session_epoch_control.rs"]
mod epoch_control_tests;
#[cfg(test)]
#[path = "session_epoch.rs"]
mod epoch_tests;
#[cfg(test)]
#[path = "session_ending.rs"]
mod ending_tests;
#[cfg(test)]
#[path = "session_user_termination.rs"]
mod user_termination_tests;
#[cfg(test)]
#[path = "session_ending_adversarial.rs"]
mod ending_adversarial_tests;
#[cfg(test)]
#[path = "session_ending_observer.rs"]
mod ending_observer_tests;
#[cfg(test)]
#[path = "session_ending_support.rs"]
mod ending_support;
#[cfg(test)]
#[path = "session_failures.rs"]
mod failure_tests;
#[cfg(test)]
#[path = "session_fragmentation.rs"]
mod fragmentation_tests;
#[cfg(test)]
#[path = "session_maintenance.rs"]
mod maintenance_tests;
#[cfg(test)]
#[path = "session_pending_input_recovery.rs"]
mod pending_input_recovery_tests;
#[cfg(test)]
#[path = "session_recovery.rs"]
mod recovery_tests;
#[cfg(test)]
#[path = "session_resume_cursor.rs"]
mod resume_cursor_tests;
#[cfg(test)]
#[path = "session_rejection_recovery.rs"]
mod rejection_recovery_tests;
#[cfg(test)]
#[path = "session_readiness.rs"]
mod readiness_tests;
#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
