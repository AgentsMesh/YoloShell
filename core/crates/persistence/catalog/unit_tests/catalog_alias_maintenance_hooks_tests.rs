use super::catalog_alias_maintenance_hooks::*;
use super::secure_vfs_alias_cleanup::CleanupIoPoint;
use std::path::Path;
use std::sync::{Arc, Barrier};

const FIRST_PATH: &str = "/first-hook-owner";
const SECOND_PATH: &str = "/second-hook-owner";
const THREAD_COUNT: usize = 2;

#[test]
fn remove_hooks_are_isolated_by_owner_thread() {
    run_path_hook_threads(set_remove_hook, run_remove_hook);
}

#[test]
fn retain_hooks_are_isolated_by_owner_thread() {
    run_path_hook_threads(set_retain_hook, run_retain_hook);
}

fn run_path_hook_threads(install: fn(Box<dyn FnOnce(&Path)>), run: fn(&Path)) {
    let barrier = Arc::new(Barrier::new(THREAD_COUNT));
    let threads: Vec<_> = [FIRST_PATH, SECOND_PATH]
        .into_iter()
        .map(|expected| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                install(Box::new(move |actual| {
                    assert_eq!(actual, Path::new(expected))
                }));
                barrier.wait();
                run(Path::new(expected));
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn cleanup_points_are_isolated_by_owner_thread() {
    let barrier = Arc::new(Barrier::new(THREAD_COUNT));
    let threads: Vec<_> = [CleanupIoPoint::EntryRemove, CleanupIoPoint::ParentSync]
        .into_iter()
        .map(|point| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                set_cleanup_io_hook(point);
                barrier.wait();
                assert!(run_cleanup_io_hook(point).is_err());
                assert!(run_cleanup_io_hook(point).is_ok());
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn mismatched_cleanup_point_does_not_consume_injection() {
    set_cleanup_io_hook(CleanupIoPoint::ParentSync);
    assert!(run_cleanup_io_hook(CleanupIoPoint::EntryRemove).is_ok());
    assert!(run_cleanup_io_hook(CleanupIoPoint::ParentSync).is_err());
    assert!(run_cleanup_io_hook(CleanupIoPoint::ParentSync).is_ok());
}
