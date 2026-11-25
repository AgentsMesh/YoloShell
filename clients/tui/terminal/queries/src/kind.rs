use crate::constants::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryKind {
    Status,
    Cursor,
    Attributes,
}

impl QueryKind {
    pub fn recognize<'a>(
        mut params: impl Iterator<Item = &'a [u16]>,
        final_byte: char,
    ) -> Option<Self> {
        // vte supplies one scalar zero for both CSI c and CSI 0 c. Accept its
        // canonical shape without inventing a default for absent parser data.
        let (Some([value]), None) = (params.next(), params.next()) else {
            return None;
        };
        match (*value, final_byte) {
            (STATUS_QUERY, STATUS_FINAL) => Some(Self::Status),
            (CURSOR_QUERY, STATUS_FINAL) => Some(Self::Cursor),
            (DEFAULT_ATTRIBUTE_QUERY, ATTRIBUTE_FINAL) => Some(Self::Attributes),
            _ => None,
        }
    }
    pub fn reply(self, screen: &vt100::Screen) -> Vec<u8> {
        match self {
            Self::Status => STATUS_REPLY.to_vec(),
            Self::Attributes => ATTRIBUTE_REPLY.to_vec(),
            Self::Cursor => {
                let (row, column) = screen.report_cursor_position();
                format!(
                    "{CURSOR_REPLY_PREFIX}{row}{CURSOR_REPLY_SEPARATOR}{column}{CURSOR_REPLY_FINAL}"
                )
                .into_bytes()
            }
        }
    }
}
