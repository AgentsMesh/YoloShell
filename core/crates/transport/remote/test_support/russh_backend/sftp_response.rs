use russh_sftp::protocol::{File, Name, Status, StatusCode};

pub fn ok(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: String::new(),
        language_tag: String::new(),
    }
}

pub fn name(id: u32, path: String) -> Name {
    Name {
        id,
        files: vec![File::dummy(path)],
    }
}
