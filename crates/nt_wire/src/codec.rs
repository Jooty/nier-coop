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
    let (message, extra) = decode_ignoring_trailing(bytes)?;
    if extra > 0 {
        return Err(WireError::TrailingBytes { extra });
    }
    Ok(message)
}

pub fn decode_ignoring_trailing<M: GameMessage>(bytes: &[u8]) -> Result<(M, usize), WireError> {
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
    message.validate()?;
    Ok((message, rest.len()))
}

fn map_decode_error(error: postcard::Error) -> WireError {
    match error {
        postcard::Error::DeserializeUnexpectedEnd => WireError::Truncated,
        _ => WireError::Malformed,
    }
}
