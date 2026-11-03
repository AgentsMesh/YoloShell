use crate::{EXECUTABLE_NAME, INTERNAL_COMMAND, ProcessRole};
use std::{ffi::OsString, fmt};

const MISSING_PROGRAM: &str = "missing executable argument";
const INVALID_ROLE: &str = "expected a valid internal Yolo process role";

#[derive(Debug, Eq, PartialEq)]
pub struct Invocation {
    pub role: Option<ProcessRole>,
    pub arguments: Vec<OsString>,
}

impl Invocation {
    pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, SelectionError> {
        let mut arguments = arguments.into_iter();
        arguments.next().ok_or(SelectionError(MISSING_PROGRAM))?;
        let first = arguments.next();
        let role = if first.as_deref() == Some(INTERNAL_COMMAND.as_ref()) {
            Some(
                arguments
                    .next()
                    .as_deref()
                    .and_then(ProcessRole::parse)
                    .ok_or(SelectionError(INVALID_ROLE))?,
            )
        } else {
            None
        };
        let mut remaining = vec![match role {
            Some(role) => role.display_name().into(),
            None => EXECUTABLE_NAME.into(),
        }];
        if role.is_none() {
            remaining.extend(first);
        }
        remaining.extend(arguments);
        Ok(Self {
            role,
            arguments: remaining,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SelectionError(&'static str);

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for SelectionError {}
