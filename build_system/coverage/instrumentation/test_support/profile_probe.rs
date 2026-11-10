mod constants;
use constants::*;
use std::io::Write;

// Stable symbols let the test inspect LLVM's structured export without a demangler.
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn yolo_coverage_probe_observed() -> bool {
    std::hint::black_box(true)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn yolo_coverage_probe_unobserved() -> bool {
    std::hint::black_box(false)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn yolo_coverage_probe_interrupted_loop() {
    for value in std::hint::black_box(vec![true, false]) {
        if std::hint::black_box(value) {
            std::hint::black_box(yolo_coverage_probe_observed());
            std::io::stdout().write_all(READY).unwrap();
            std::io::stdout().flush().unwrap();
            loop {
                std::thread::park();
            }
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn yolo_coverage_probe_loop_exit(panics: bool) -> Result<(), ()> {
    for value in std::hint::black_box(vec![true, false]) {
        if std::hint::black_box(value) {
            std::hint::black_box(yolo_coverage_probe_observed());
            if panics {
                panic!("{INJECTED_PANIC}");
            }
            return Err(());
        }
    }
    Ok(())
}

fn main() {
    let mode = std::env::args().nth(MODE_ARGUMENT).expect(MODE_REQUIRED);
    if mode == IN_LOOP {
        yolo_coverage_probe_interrupted_loop();
        return;
    }
    if mode == HUNG_RESULT {
        let _ = yolo_coverage_probe_error_before_loop(true);
        return;
    }
    if mode == BEFORE_LOOP {
        std::hint::black_box(yolo_coverage_probe_observed());
        assert!(yolo_coverage_probe_error_before_loop(false).is_err());
    } else if mode == PANIC_LOOP {
        let _ = std::panic::catch_unwind(|| yolo_coverage_probe_loop_exit(true));
    } else if mode == RETURN_LOOP {
        assert!(yolo_coverage_probe_loop_exit(false).is_err());
    } else if mode == THREADED {
        let started = std::sync::Barrier::new(THREAD_COUNT);
        std::thread::scope(|scope| {
            for _ in 0..THREAD_COUNT {
                scope.spawn(|| {
                    started.wait();
                    for _ in 0..CALLS_PER_THREAD {
                        std::hint::black_box(yolo_coverage_probe_observed());
                    }
                });
            }
        });
    } else {
        std::hint::black_box(if mode == COLD {
            yolo_coverage_probe_unobserved()
        } else {
            yolo_coverage_probe_observed()
        });
    }
    std::io::stdout().write_all(READY).unwrap();
    std::io::stdout().flush().unwrap();
    if matches!(
        mode.as_str(),
        BLOCKED | THREADED | PANIC_LOOP | RETURN_LOOP | BEFORE_LOOP
    ) {
        loop {
            std::thread::park();
        }
    }
    assert_eq!(mode, NORMAL);
}

#[inline(never)]
fn probe_check(accepted: bool, blocked: bool) -> anyhow::Result<bool> {
    if std::hint::black_box(blocked) {
        std::hint::black_box(yolo_coverage_probe_observed());
        std::io::stdout().write_all(READY).unwrap();
        std::io::stdout().flush().unwrap();
        loop {
            std::thread::park();
        }
    }
    anyhow::ensure!(std::hint::black_box(accepted), INJECTED_PANIC);
    Ok(true)
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn yolo_coverage_probe_error_before_loop(blocked: bool) -> anyhow::Result<Vec<bool>> {
    probe_check(false, blocked)?;
    let mut holders = Vec::new();
    for value in std::hint::black_box(vec![true, false]) {
        anyhow::ensure!(probe_check(value, false)?, INJECTED_PANIC);
        let matched = probe_check(value, false)?;
        anyhow::ensure!(matched, INJECTED_PANIC);
        anyhow::ensure!(probe_check(value, false)?, INJECTED_PANIC);
        holders.push(value);
    }
    Ok(holders)
}
