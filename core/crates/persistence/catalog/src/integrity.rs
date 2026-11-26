use super::*;

impl Catalog {
    pub fn integrity_check(&self) -> Result<()> {
        let result: String =
            self.connection
                .query_row(constants::INTEGRITY_CHECK_SQL, [], |row| {
                    row.get(constants::QUICK_CHECK_COLUMN)
                })?;
        if result != constants::INTEGRITY_OK {
            return Err(rebuild_reason::integrity(result));
        }
        Ok(())
    }
}
