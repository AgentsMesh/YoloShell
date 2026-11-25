//! Public error adaptation for platform dispatch. The mapping itself lives in
//! the shared local-errors package so every local entry point shares one
//! translation.
pub use yolo_hub_local_errors::map_error;
