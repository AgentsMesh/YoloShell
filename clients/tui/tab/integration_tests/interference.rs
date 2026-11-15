use crate::support::*;
use yolo_tui_tab::*;

#[test]
fn external_replacement_poisoned_save_preserves_the_replacement() {
    let (_root, path, hub) = fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    let mut replacement = store.document().clone();
    replacement.tabs.update_active(LocalTabView {
        layout: layout(),
        view: TabViewMode::Inventory,
    });
    yolo_paths::atomic_write_json(&path.join(TAB_FILE), &replacement).unwrap();
    let desired = LocalTabView {
        layout: layout(),
        view: TabViewMode::Terminal,
    };
    assert!(store.save(&desired).is_err());
    assert!(store.directory().is_err());
    assert!(store.save(&desired).is_err());
    let current: TabDocument = yolo_paths::read_json(&path.join(TAB_FILE)).unwrap();
    assert_eq!(current, replacement);
}

#[test]
fn renamed_namespace_and_deleted_file_cannot_be_recreated_by_an_old_owner() {
    let (root, path, hub) = fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    std::fs::rename(&path, root.path().join(MOVED)).unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(&path).unwrap();
    assert!(store.directory().is_err());
    assert!(store.save(&LocalTabView::default()).is_err());
    assert!(!path.join(TAB_FILE).exists());
    let (_root, path, hub) = fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    std::fs::remove_file(path.join(TAB_FILE)).unwrap();
    assert!(store.directory().is_err());
    assert!(store.save(&LocalTabView::default()).is_err());
    assert!(!path.join(TAB_FILE).exists());
}

#[test]
fn symlinked_tab_file_does_not_redirect_a_write() {
    let (root, path, hub) = fixture();
    let mut store = TabStore::open(&path, hub).unwrap();
    let other = root.path().join(REPLACEMENT);
    std::fs::write(&other, INVALID_BYTES).unwrap();
    std::fs::remove_file(path.join(TAB_FILE)).unwrap();
    std::os::unix::fs::symlink(&other, path.join(TAB_FILE)).unwrap();
    assert!(store.save(&LocalTabView::default()).is_err());
    assert_eq!(std::fs::read(other).unwrap(), INVALID_BYTES);
}
