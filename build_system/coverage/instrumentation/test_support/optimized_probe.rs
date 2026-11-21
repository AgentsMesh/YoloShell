mod constants;
mod layout_marker;
use constants::*;
use std::io::Write;

#[unsafe(no_mangle)]
#[inline(never)]
pub fn yolo_coverage_layout_observed(accepted: bool) -> u64 {
    if std::hint::black_box(accepted) {
        std::hint::black_box(O3_RESULT)
    } else {
        std::hint::black_box(NOT_EXECUTED)
    }
}

fn main() {
    let mode = std::env::args().nth(MODE_ARGUMENT).expect(MODE_REQUIRED);
    layout_marker::verify_and_advance();
    let mut total = NOT_EXECUTED;
    for _ in 0..O3_CALLS {
        total += yolo_coverage_layout_observed(true);
    }
    assert_eq!(total, O3_CALLS * O3_RESULT);
    std::io::stdout().write_all(READY).unwrap();
    std::io::stdout().flush().unwrap();
    if mode == BLOCKED {
        loop {
            std::thread::park();
        }
    }
    assert_eq!(mode, NORMAL);
}
