use crate::{ScopedDispatcher, admission::visible, constants::MAX_CATALOGUE_SCAN_PAGES};
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_hub_terminal_protocol::PublicError;

impl ScopedDispatcher {
    /// Scan bounded internal pages, exposing only authorized rows and cursors.
    /// A hidden row's identity must not escape through an otherwise empty page.
    pub(crate) async fn page<T, K: Copy + Ord>(
        &self,
        mut after: Option<K>,
        limit: usize,
        command: impl Fn(Option<K>) -> Command,
        extract: impl Fn(Reply) -> Result<(Vec<T>, Option<K>), PublicError>,
        key: impl Fn(&T) -> K,
        origin: impl Fn(&T) -> HubId,
    ) -> Result<(Vec<T>, Option<K>), PublicError> {
        let mut visible_rows = Vec::new();
        for _ in 0..MAX_CATALOGUE_SCAN_PAGES {
            crate::guard::verify(&self.admission)?;
            let request = Request::new(self.hub_id(), command(after))?;
            let Operation::Reply(reply) = self.inner.dispatch(request.clone()).await? else {
                return Err(PublicError::InvalidPayload);
            };
            // Shared response validation checks ordering, exact page cursors and
            // every row advancing past this request's cursor before filtering.
            Response::new(&request, self.hub_id(), reply.clone()).validate_for(&request)?;
            if let Reply::Error { code } = reply {
                return Err(code);
            }
            let (rows, next) = extract(reply)?;
            for row in rows {
                if visible(&self.admission, origin(&row)) {
                    if visible_rows.len() == limit {
                        let next = visible_rows.last().map(&key);
                        return Ok((visible_rows, next));
                    }
                    visible_rows.push(row);
                }
            }
            let Some(next) = next else {
                return Ok((visible_rows, None));
            };
            after = Some(next);
        }
        Err(PublicError::InvalidPayload)
    }
}
