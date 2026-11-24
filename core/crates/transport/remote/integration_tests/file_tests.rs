use super::super::*;
use crate::remote::constants::MAX_FILE_READ_BYTES;
use std::path::PathBuf;

#[test]
fn file_policy_rejects_traversal_and_symlink_escape() {
    let policy = FileReadPolicy::standard(vec![PathBuf::from("/Users/remote/work")]).unwrap();
    let traversal = FileReadRequest::new("/Users/remote/work/../secret", 0, 10);
    assert!(policy.authorize(traversal, "/Users/remote/secret").is_err());
    let symlink = FileReadRequest::new("/Users/remote/work/link", 0, 10);
    assert!(policy.authorize(symlink, "/etc/passwd").is_err());
}

#[test]
fn policy_constructor_and_request_bounds_fail_independently() {
    assert!(FileReadPolicy::new(vec![], 1).is_err());
    assert!(FileReadPolicy::new(vec![PathBuf::from("/srv")], 0).is_err());
    assert!(FileReadPolicy::new(vec![PathBuf::from("/srv")], MAX_FILE_READ_BYTES + 1).is_err());
    assert!(FileReadPolicy::new(vec![PathBuf::from("relative")], 1).is_err());
    let policy = FileReadPolicy::new(vec![PathBuf::from("/srv/project")], 8).unwrap();
    for request in [
        FileReadRequest::new("relative", 0, 1),
        FileReadRequest::new("/srv/other", 0, 1),
        FileReadRequest::new("/srv/project/file", 0, 0),
        FileReadRequest::new("/srv/project/file", 0, 9),
        FileReadRequest::new("/srv/project/file", u64::MAX, 1),
    ] {
        assert!(policy.authorize(request, "/srv/project/file").is_err());
    }
}

#[test]
fn resolved_path_and_result_are_both_rechecked() {
    let policy = FileReadPolicy::new(vec![PathBuf::from("/srv/project")], 8).unwrap();
    let request = FileReadRequest::new("/srv/project/file", 0, 4);
    assert!(policy.authorize(request.clone(), "relative").is_err());
    assert!(
        policy
            .authorize(request.clone(), "/srv/other/file")
            .is_err()
    );
    let authorized = policy.authorize(request, "/srv/project/file").unwrap();
    let wrong_path = FileReadResult {
        resolved_path: PathBuf::from("/srv/project/other"),
        data: vec![],
        eof: true,
    };
    assert!(wrong_path.validate(&authorized).is_err());
    let oversized = FileReadResult {
        resolved_path: PathBuf::from("/srv/project/file"),
        data: vec![0; 5],
        eof: false,
    };
    assert!(oversized.validate(&authorized).is_err());
}

#[test]
fn absolute_path_parser_rejects_every_lexical_escape() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    assert!(super::super::file::strict_absolute(std::path::Path::new(
        "/"
    )));
    for value in ["relative", "/srv//file", "/srv/./file", "/srv/../file"] {
        assert!(!super::super::file::strict_absolute(std::path::Path::new(
            value
        )));
    }
    let invalid = PathBuf::from(OsString::from_vec(vec![b'/', 0xff]));
    assert!(!super::super::file::strict_absolute(&invalid));
}

#[test]
fn file_policy_uses_component_boundaries_and_limits() {
    let policy = FileReadPolicy::new(vec![PathBuf::from("/srv/project")], 32).unwrap();
    assert!(
        policy
            .authorize(
                FileReadRequest::new("/srv/project-other/file", 0, 4),
                "/srv/project-other/file"
            )
            .is_err()
    );
    let read = policy
        .authorize(
            FileReadRequest::new("/srv/project/file", 4, 8),
            "/srv/project/file",
        )
        .unwrap();
    assert_eq!(read.request().offset(), 4);
}
