use super::InventoryEntry;
use std::collections::{HashMap, HashSet, VecDeque};

const FIXTURE_ALREADY_INSTALLED_DETAIL: &str = "process inventory fixture is already installed";
const FIXTURE_EXHAUSTED_DETAIL: &str = "process inventory fixture exhausted";
const FIXTURE_NOT_INSTALLED_DETAIL: &str = "process inventory fixture is not installed";

enum TestInventorySource {
    Queue(VecDeque<std::io::Result<Vec<InventoryEntry>>>),
    Repeating(Vec<InventoryEntry>),
}

thread_local! {
    static TEST_INVENTORY: std::cell::RefCell<Option<TestInventorySource>> =
        const { std::cell::RefCell::new(None) };
}

pub(crate) struct TestInventoryGuard(std::marker::PhantomData<std::rc::Rc<()>>);

pub(crate) fn install_test_inventory(
    snapshots: impl IntoIterator<Item = Vec<InventoryEntry>>,
) -> TestInventoryGuard {
    install_test_inventory_results(snapshots.into_iter().map(Ok))
}

pub(crate) fn install_test_inventory_results(
    snapshots: impl IntoIterator<Item = std::io::Result<Vec<InventoryEntry>>>,
) -> TestInventoryGuard {
    install_test_inventory_source(TestInventorySource::Queue(snapshots.into_iter().collect()))
}

pub(crate) fn install_repeating_test_inventory(
    snapshot: Vec<InventoryEntry>,
) -> TestInventoryGuard {
    install_test_inventory_source(TestInventorySource::Repeating(snapshot))
}

fn install_test_inventory_source(source: TestInventorySource) -> TestInventoryGuard {
    TEST_INVENTORY.with(|slot| {
        let mut current = slot.borrow_mut();
        assert!(current.is_none(), "{}", FIXTURE_ALREADY_INSTALLED_DETAIL);
        *current = Some(source);
    });
    TestInventoryGuard(std::marker::PhantomData)
}

impl TestInventoryGuard {
    pub(crate) fn replace_repeating(&mut self, snapshot: Vec<InventoryEntry>) {
        TEST_INVENTORY.with(|slot| {
            let mut source = slot.borrow_mut();
            assert!(source.is_some(), "{}", FIXTURE_NOT_INSTALLED_DETAIL);
            *source = Some(TestInventorySource::Repeating(snapshot));
        });
    }
}

pub(super) fn take_test_inventory() -> Option<std::io::Result<Vec<InventoryEntry>>> {
    TEST_INVENTORY.with(|slot| {
        let mut source = slot.borrow_mut();
        match source.as_mut()? {
            TestInventorySource::Queue(snapshots) => {
                Some(snapshots.pop_front().expect(FIXTURE_EXHAUSTED_DETAIL))
            }
            TestInventorySource::Repeating(snapshot) => Some(Ok(snapshot
                .iter()
                .copied()
                .filter(|entry| entry.identity().matches())
                .collect())),
        }
    })
}

pub(crate) fn parent_map(entries: &[InventoryEntry]) -> HashMap<u32, u32> {
    entries
        .iter()
        .map(|entry| (entry.pid, entry.parent_pid))
        .collect()
}

pub(crate) fn is_descendant(pid: u32, root: u32, parents: &HashMap<u32, u32>) -> bool {
    let mut current = pid;
    let mut visited = HashSet::new();
    while let Some(parent) = parents.get(&current).copied() {
        if parent == root {
            return true;
        }
        if parent == current || !visited.insert(current) {
            return false;
        }
        current = parent;
    }
    false
}

impl Drop for TestInventoryGuard {
    fn drop(&mut self) {
        TEST_INVENTORY.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(feature = "yolo-integration-tests")]
#[path = "../../integration_tests/inventory/inventory_snapshot_fixture_tests.rs"]
mod tests;
