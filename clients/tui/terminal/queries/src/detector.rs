use crate::kind::QueryKind;

#[derive(Default)]
pub(crate) struct QueryDetector {
    pub query: Option<QueryKind>,
}

impl vte::Perform for QueryDetector {
    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        ignore: bool,
        final_byte: char,
    ) {
        if !ignore && intermediates.is_empty() {
            self.query = QueryKind::recognize(params.iter(), final_byte);
        }
    }
    fn terminated(&self) -> bool {
        self.query.is_some()
    }
}
