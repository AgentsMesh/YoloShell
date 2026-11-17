macro_rules! directory_handlers {
    () => {
        async fn mkdir(
            &mut self,
            id: u32,
            path: String,
            attrs: FileAttributes,
        ) -> Result<Status, Self::Error> {
            attributes::reject_unintended_mutations(&attrs)?;
            self.filesystem.mkdir(&path, attrs.permissions)?;
            Ok(ok(id))
        }
        async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
            self.filesystem.rmdir(&path)?;
            Ok(ok(id))
        }
    };
}

pub(super) use directory_handlers;
