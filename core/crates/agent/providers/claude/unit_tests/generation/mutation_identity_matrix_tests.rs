use super::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

const HOME: &str = "/generation-test";
const INVALID_COMPONENT: &str = "invalid";
const INVALID_UTF8: &[u8] = &[0xff];

#[test]
fn managed_authority_recognition_requires_exact_depth_session_directory_and_filename() {
    let session_id = Uuid::new_v4();
    let paths = Paths::new(PathBuf::from(HOME));
    let valid_path = paths.session_dir(session_id).join(AUTHORITY_FILE_NAME);
    let store = ClaudeGenerationStore {
        path: valid_path.clone(),
    };
    let (observed, observed_id) = managed_session(&store).unwrap().unwrap();
    assert_eq!(observed.home(), paths.home());
    assert_eq!(observed_id, session_id);
    let mut invalid_utf8 = paths.sessions_dir();
    invalid_utf8.push(OsString::from_vec(INVALID_UTF8.to_vec()));
    invalid_utf8.push(AUTHORITY_FILE_NAME);
    let invalid = [
        PathBuf::new(),
        PathBuf::from("/"),
        PathBuf::from("/authority"),
        PathBuf::from("/session/authority"),
        paths
            .sessions_dir()
            .join(INVALID_COMPONENT)
            .join(AUTHORITY_FILE_NAME),
        paths
            .home()
            .join(INVALID_COMPONENT)
            .join(session_id.to_string())
            .join(AUTHORITY_FILE_NAME),
        paths
            .sessions_dir()
            .join(session_id.simple().to_string())
            .join(AUTHORITY_FILE_NAME),
        valid_path.with_file_name(INVALID_COMPONENT),
        invalid_utf8,
    ];
    for path in invalid {
        assert!(
            managed_session(&ClaudeGenerationStore { path: path.clone() })
                .unwrap()
                .is_none(),
            "{path:?}"
        );
    }
}
