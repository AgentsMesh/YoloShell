use crate::Value;

pub(crate) struct Pending {
    pub value: Value,
    cleanup: fn(&mut Value),
}
impl Pending {
    pub fn new(value: Value, cleanup: fn(&mut Value)) -> Self {
        Self { value, cleanup }
    }
    pub fn take(mut self) -> Value {
        std::mem::replace(&mut self.value, Value::Null)
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        (self.cleanup)(&mut self.value);
    }
}
