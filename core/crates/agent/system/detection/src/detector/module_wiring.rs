#[path = "detection.rs"]
mod detection;
#[path = "evidence.rs"]
mod evidence;
#[path = "transitions.rs"]
mod transitions;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../../unit_tests/detector/module_wiring.rs");
