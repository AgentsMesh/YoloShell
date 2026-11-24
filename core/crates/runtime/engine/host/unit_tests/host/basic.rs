use crate::host::tests::*;
use super::*;

    #[test]
    fn command_basename_is_stable() {
        assert_eq!(command_basename("/usr/local/bin/codex"), crate::agent::codex::constants::EXECUTABLE);
    }

    #[test]
    fn capabilities_are_unique_and_sorted() {
        let values = capabilities();
        let mut sorted = values.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(values, sorted);
        assert!(values.iter().any(|value| {
            value == crate::protocol::capability::IDLE_HOST_SHUTDOWN
        }));
    }

    #[test]
    fn cwd_expansion_uses_the_host_home_and_rejects_ambiguous_forms() {
        let directory = tempfile::tempdir().unwrap();
        let child = directory.path().join("project");
        std::fs::create_dir(&child).unwrap();
        assert_eq!(
            resolve_cwd(Path::new("~"), Some(directory.path())).unwrap(),
            directory.path().canonicalize().unwrap()
        );
        assert_eq!(
            resolve_cwd(Path::new("~/project"), Some(directory.path())).unwrap(),
            child.canonicalize().unwrap()
        );
        assert!(resolve_cwd(Path::new("~other/project"), Some(directory.path())).is_err());
        assert!(resolve_cwd(Path::new("relative"), Some(directory.path())).is_err());
        assert!(resolve_cwd(Path::new("~"), None).is_err());
        let file = directory.path().join("file");
        std::fs::write(&file, b"not a directory").unwrap();
        assert!(resolve_cwd(&file, Some(directory.path())).is_err());
        assert!(resolve_cwd(&directory.path().join("missing"), Some(directory.path())).is_err());
    }
