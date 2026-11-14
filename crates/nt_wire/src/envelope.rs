use crate::{GameMessage, WireError, decode, decode_ignoring_trailing, encode};

pub const ENVELOPE_BYTES: usize = 2;
pub const ADDRESSED_TO_EVERYONE: u8 = 0xFF;

/// Who a frame after the handshake is from, and which player it is for (`None`: everyone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Envelope {
    pub from: u8,
    pub to: Option<u8>,
}

/// Writes `[from, to]` in front of the encoded message, `to = None` as [`ADDRESSED_TO_EVERYONE`].
pub fn encode_enveloped<M: GameMessage>(
    envelope: &Envelope,
    message: &M,
) -> Result<Vec<u8>, WireError> {
    frame_enveloped(envelope, &encode(message)?)
}

/// Writes `[from, to]` in front of `body`, which is sent as it is.
pub fn frame_enveloped(envelope: &Envelope, body: &[u8]) -> Result<Vec<u8>, WireError> {
    if envelope.from == ADDRESSED_TO_EVERYONE {
        return Err(WireError::OutOfRange { field: "from" });
    }
    let to = match envelope.to {
        None => ADDRESSED_TO_EVERYONE,
        Some(ADDRESSED_TO_EVERYONE) => return Err(WireError::OutOfRange { field: "to" }),
        Some(slot) => slot,
    };
    let mut frame = Vec::with_capacity(ENVELOPE_BYTES + body.len());
    frame.push(envelope.from);
    frame.push(to);
    frame.extend_from_slice(body);
    Ok(frame)
}

/// Reads a frame written by [`encode_enveloped`]; the body keeps every limit of [`decode`].
pub fn decode_enveloped<M: GameMessage>(frame: &[u8]) -> Result<(Envelope, M), WireError> {
    let (envelope, body) = split_envelope(frame)?;
    Ok((envelope, decode(body)?))
}

/// Like [`decode_enveloped`], but bytes after the message are counted, not refused.
pub fn decode_enveloped_ignoring_trailing<M: GameMessage>(
    frame: &[u8],
) -> Result<(Envelope, M, usize), WireError> {
    let (envelope, body) = split_envelope(frame)?;
    let (message, extra) = decode_ignoring_trailing(body)?;
    Ok((envelope, message, extra))
}

fn split_envelope(frame: &[u8]) -> Result<(Envelope, &[u8]), WireError> {
    if frame.is_empty() {
        return Err(WireError::Empty);
    }
    let [from, to, body @ ..] = frame else {
        return Err(WireError::Truncated);
    };
    if *from == ADDRESSED_TO_EVERYONE {
        return Err(WireError::OutOfRange { field: "from" });
    }
    let envelope = Envelope {
        from: *from,
        to: (*to != ADDRESSED_TO_EVERYONE).then_some(*to),
    };
    Ok((envelope, body))
}
