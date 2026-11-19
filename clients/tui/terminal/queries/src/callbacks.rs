use crate::kind::QueryKind;

/// vt100 confirms a query against the continuously parsed terminal screen.
#[derive(Default)]
pub struct QueryCallbacks {
    expected: Option<QueryKind>,
    reply: Option<Vec<u8>>,
    synchronized_output: bool,
}

impl QueryCallbacks {
    pub(crate) fn arm(&mut self, expected: QueryKind) {
        self.expected = Some(expected);
        self.reply = None;
    }
    pub(crate) fn clear(&mut self) {
        self.expected = None;
        self.reply = None;
    }
    pub(crate) fn take(&mut self) -> Option<Vec<u8>> {
        self.expected = None;
        self.reply.take()
    }
    /// The program requested atomic presentation (DEC private mode 2026).
    /// The parser continues to consume bytes, but a renderer should not paint
    /// the intermediate states between the set and reset.
    pub fn synchronized_output(&self) -> bool {
        self.synchronized_output
    }
}

impl vt100::Callbacks for QueryCallbacks {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        first: Option<u8>,
        second: Option<u8>,
        params: &[&[u16]],
        final_byte: char,
    ) {
        if first == Some(b'?') && second.is_none() {
            if params.iter().any(|parameter| parameter.contains(&2026)) {
                match final_byte {
                    'h' => self.synchronized_output = true,
                    'l' => self.synchronized_output = false,
                    _ => {}
                }
                return;
            }
            return;
        }
        if (first, second) != (None, None) {
            return;
        }
        let Some(query) = QueryKind::recognize(params.iter().copied(), final_byte) else {
            return;
        };
        if self.expected.take() == Some(query) {
            self.reply = Some(query.reply(screen));
        }
    }
}
