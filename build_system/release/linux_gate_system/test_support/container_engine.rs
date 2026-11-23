use crate::container::engine::{Engine, Selector};
use crate::container::model::Inspection;
use anyhow::{Result, bail};
use std::cell::{Cell, RefCell};

pub const ENGINE_ID: &str = "fixture-engine-identity";
pub const OTHER_ENGINE: &str = "replaced-engine-identity";
pub const CONTAINER_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const CLIENT_FAILURE: &str = "fixture Docker CLI timed out";

#[derive(Debug, Eq, PartialEq)]
pub enum Lookup {
    Name(String),
    Id(String),
}

#[derive(Default)]
pub struct TestEngine {
    pub current: RefCell<Option<Inspection>>,
    pub lookups: RefCell<Vec<Lookup>>,
    pub removals: RefCell<Vec<String>>,
    pub fail_remove: Cell<bool>,
    pub fail_lookup: Cell<bool>,
    pub replaced: Cell<bool>,
}
impl Engine for TestEngine {
    fn identity(&self) -> Result<String> {
        Ok(if self.replaced.get() {
            OTHER_ENGINE
        } else {
            ENGINE_ID
        }
        .into())
    }
    fn find(&self, selector: Selector<'_>) -> Result<Option<Inspection>> {
        self.lookups.borrow_mut().push(match selector {
            Selector::Name(name) => Lookup::Name(name.into()),
            Selector::Id(id) => Lookup::Id(id.into()),
        });
        if self.fail_lookup.get() {
            bail!("fixture daemon lookup failed");
        }
        if let Some(current) = self.current.borrow().as_ref() {
            match selector {
                Selector::Id(id) => assert_eq!(id, current.id),
                Selector::Name(name) => assert_eq!(format!("/{name}"), current.name),
            }
        }
        Ok(self.current.borrow().clone())
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.removals.borrow_mut().push(id.into());
        if self.fail_remove.get() {
            self.fail_lookup.set(true);
            bail!("fixture daemon removal failed");
        }
        assert_eq!(self.current.borrow().as_ref().unwrap().id, id);
        self.current.borrow_mut().take();
        Ok(())
    }
}
