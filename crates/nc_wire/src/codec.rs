use crate::{GameMessage, MAX_MESSAGE_BYTES, WireError};

pub fn encode<M: GameMessage>(message: &M) -> Result<Vec<u8>, WireError> {
    message.validate()?;
    let bytes = postcard::to_stdvec(message).map_err(|_| WireError::Malformed)?;
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(WireError::TooLarge {
            len: bytes.len(),
            limit: MAX_MESSAGE_BYTES,
        });
    }
    Ok(bytes)
}

pub fn decode<M: GameMessage>(bytes: &[u8]) -> Result<M, WireError> {
    if bytes.is_empty() {
        return Err(WireError::Empty);
    }
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(WireError::TooLarge {
            len: bytes.len(),
            limit: MAX_MESSAGE_BYTES,
        });
    }
    let (message, rest) = postcard::take_from_bytes::<M>(bytes).map_err(map_decode_error)?;
    if !rest.is_empty() {
        return Err(WireError::TrailingBytes { extra: rest.len() });
    }
    message.validate()?;
    Ok(message)
}

fn map_decode_error(error: postcard::Error) -> WireError {
    match error {
        postcard::Error::DeserializeUnexpectedEnd => WireError::Truncated,
        _ => WireError::Malformed,
    }
}
