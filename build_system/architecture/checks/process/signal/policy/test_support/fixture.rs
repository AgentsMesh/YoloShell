use std::cell::RefCell;
use std::collections::BTreeMap;
use yolo_architecture_foundation::SourceFile;

pub struct Fixture {
    files: RefCell<BTreeMap<String, String>>,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            files: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let text = String::from_utf8(contents.as_ref().to_vec()).unwrap();
        self.files.borrow_mut().insert(relative.to_owned(), text);
    }

    pub fn sources(&self) -> Vec<SourceFile> {
        self.files
            .borrow()
            .iter()
            .map(|(relative, text)| SourceFile {
                absolute: relative.clone().into(),
                relative: relative.clone(),
                text: text.clone(),
            })
            .collect()
    }
}
