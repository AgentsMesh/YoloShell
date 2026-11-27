use crate::*;
use std::ffi::{OsStr, OsString};

const ROLE_ARGUMENT_COUNT: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessRole {
    Host,
    Hub,
    Attach,
    Remote,
    HubAttach,
    FileHelper,
}

impl ProcessRole {
    pub const ALL: &[Self] = &[
        Self::Host,
        Self::Hub,
        Self::Attach,
        Self::Remote,
        Self::HubAttach,
        Self::FileHelper,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Host => HOST_ROLE,
            Self::Hub => HUB_ROLE,
            Self::Attach => ATTACH_ROLE,
            Self::Remote => REMOTE_ROLE,
            Self::HubAttach => HUB_ATTACH_ROLE,
            Self::FileHelper => FILE_HELPER_ROLE,
        }
    }

    pub const fn arguments(self) -> [&'static str; ROLE_ARGUMENT_COUNT] {
        [INTERNAL_COMMAND, self.name()]
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Host => HOST_ROLE_DISPLAY,
            Self::Hub => HUB_ROLE_DISPLAY,
            Self::Attach => ATTACH_ROLE_DISPLAY,
            Self::Remote => REMOTE_ROLE_DISPLAY,
            Self::HubAttach => HUB_ATTACH_ROLE_DISPLAY,
            Self::FileHelper => FILE_HELPER_ROLE_DISPLAY,
        }
    }

    pub fn parse(value: &OsStr) -> Option<Self> {
        Self::ALL.iter().copied().find(|role| value == role.name())
    }

    pub fn prepend(self, arguments: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
        self.arguments()
            .into_iter()
            .map(OsString::from)
            .chain(arguments)
            .collect()
    }
}
