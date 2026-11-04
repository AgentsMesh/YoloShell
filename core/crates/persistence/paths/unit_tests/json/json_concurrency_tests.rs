use super::super::*;
use std::sync::{Arc, Barrier};

const CONCURRENT_WRITERS: usize = 4;
const COORDINATOR_COUNT: usize = 1;

#[test]
fn independent_targets_in_one_directory_write_concurrently() {
    let root = tempfile::tempdir().unwrap();
    create_private_dir(root.path()).unwrap();
    let barrier = Arc::new(Barrier::new(CONCURRENT_WRITERS + COORDINATOR_COUNT));
    let writers = (0..CONCURRENT_WRITERS)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            let path = root.path().join(format!("state-{index}.json"));
            std::thread::spawn(move || {
                barrier.wait();
                atomic_write_json(&path, &serde_json::json!({"writer": index}))?;
                read_json::<serde_json::Value>(&path)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();

    for (index, writer) in writers.into_iter().enumerate() {
        let value = writer.join().unwrap().unwrap();
        assert_eq!(value["writer"], index);
    }
}
