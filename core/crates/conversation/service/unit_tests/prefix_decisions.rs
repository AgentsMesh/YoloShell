use crate::prefix;
use std::{fs::OpenOptions, io::Write, os::unix::fs::FileExt};
use yolo_conversation_protocol::ConversationError;

const ORIGINAL: &[u8] = b"same native conversation\n";
const APPENDED: &[u8] = b"new record\n";

enum Change {
    IdentityOnly,
    Stable,
    Append,
    Rewrite,
}

#[test]
fn one_snapshot_consumer_distinguishes_no_content_stable_append_and_rewritten_prefix() {
    for change in [
        Change::IdentityOnly,
        Change::Stable,
        Change::Append,
        Change::Rewrite,
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().write_all(ORIGINAL).unwrap();
        let result = prefix::guard(file.as_file(), ORIGINAL.len() as u64, |source, length| {
            if matches!(change, Change::IdentityOnly) {
                return Ok(((), None));
            }
            let digest = prefix::hash(source, length)?;
            match change {
                Change::Append => OpenOptions::new()
                    .append(true)
                    .open(file.path())?
                    .write_all(APPENDED)?,
                Change::Rewrite => source.write_all_at(b"other", 0)?,
                Change::Stable | Change::IdentityOnly => {}
            }
            Ok(((), Some(digest)))
        });
        if matches!(change, Change::Rewrite) {
            assert_eq!(
                result.unwrap_err().downcast_ref::<ConversationError>(),
                Some(&ConversationError::EvidenceChanged)
            );
        } else {
            result.unwrap();
        }
    }
}
