use yolo_release_delivery_foundation::delivery_stage::DeliveryStage;
use yolo_release_filesystem::DirectoryHandle;

const USAGE: &str = "unsettled delivery extraction";

#[test]
fn unsettled_or_active_stage_use_prevents_drop_cleanup() {
    for unresolved in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let parent = DirectoryHandle::open(root.path()).unwrap();
        let stage = DeliveryStage::create(&parent).unwrap();
        let path = stage.handle().display_path().to_owned();
        let usage = stage.handle().retain_resource_use(USAGE).unwrap();
        if unresolved {
            drop(usage);
            drop(stage);
        } else {
            drop(stage);
            usage.confirm_released();
        }
        assert!(path.is_dir());
    }
}

#[test]
fn confirmed_stage_use_allows_cleanup_but_prepared_policy_still_preserves() {
    for prepared in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let parent = DirectoryHandle::open(root.path()).unwrap();
        let mut stage = DeliveryStage::create(&parent).unwrap();
        let path = stage.handle().display_path().to_owned();
        stage
            .handle()
            .retain_resource_use(USAGE)
            .unwrap()
            .confirm_released();
        if prepared {
            stage.retain();
        }
        drop(stage);
        assert_eq!(path.exists(), prepared);
    }
}
