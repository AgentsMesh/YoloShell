use crate::{FixtureError, constants::*, lines};
use tokio::io::BufReader;

#[tokio::test]
async fn framed_input_is_bounded_and_preserves_the_next_request() {
    let mut bytes = vec![FILL_BYTE; MAX_LINE_BYTES];
    bytes.extend_from_slice(LINE_TERMINATOR);
    bytes.extend_from_slice(SECOND_LINE);
    let mut input = BufReader::with_capacity(INPUT_BUFFER_BYTES, bytes.as_slice());
    assert_eq!(
        lines::next(&mut input).await.unwrap().unwrap().len(),
        MAX_LINE_BYTES
    );
    assert_eq!(
        lines::next(&mut input).await.unwrap().unwrap(),
        SECOND_PAYLOAD
    );
    assert!(lines::next(&mut input).await.unwrap().is_none());
}

#[tokio::test]
async fn oversized_or_unterminated_request_never_becomes_a_publish() {
    let bytes = vec![FILL_BYTE; MAX_LINE_BYTES + DELIMITER_BYTES];
    assert!(matches!(
        lines::next(&mut BufReader::new(bytes.as_slice())).await,
        Err(FixtureError::TooLarge)
    ));
    assert!(matches!(
        lines::next(&mut BufReader::new(SECOND_PAYLOAD)).await,
        Err(FixtureError::InvalidRequest)
    ));
    assert!(
        lines::next(&mut BufReader::new(tokio::io::empty()))
            .await
            .unwrap()
            .is_none()
    );
}

const FILL_BYTE: u8 = b'x';
const SECOND_LINE: &[u8] = b"{}\n";
const SECOND_PAYLOAD: &[u8] = b"{}";
