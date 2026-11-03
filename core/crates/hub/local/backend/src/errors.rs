//! Public error adaptation for platform dispatch. The mapping itself lives in
//! the platform package so every platform entry point shares one translation.
pub(crate) use yolo_hub_local_platform::map_error;
