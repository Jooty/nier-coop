use nc_wire::{
    ADDRESSED_TO_EVERYONE, ENVELOPE_BYTES, Envelope, MAX_FRAME_BYTES, MAX_MESSAGE_BYTES, WireError,
    decode_enveloped, decode_enveloped_ignoring_trailing, encode, encode_enveloped,
    frame_enveloped,
};

use crate::tiny::Tiny;

#[test]
fn an_enveloped_message_round_trips() {
    let envelope = Envelope {
        from: 1,
        to: Some(0),
    };
    let message = Tiny::Blob(vec![4, 5]);
    let frame = encode_enveloped(&envelope, &message).expect("encodes");
    assert_eq!(decode_enveloped::<Tiny>(&frame), Ok((envelope, message)));
}

#[test]
fn the_envelope_is_from_then_to_in_front_of_the_body() {
    let message = Tiny::Blob(vec![7]);
    let body = encode(&message).expect("encodes");
    let frame = encode_enveloped(
        &Envelope {
            from: 0,
            to: Some(1),
        },
        &message,
    )
    .expect("encodes");
    assert_eq!(frame[..ENVELOPE_BYTES], [0, 1]);
    assert_eq!(frame[ENVELOPE_BYTES..], body[..]);
}

#[test]
fn a_frame_for_everyone_is_addressed_to_0xff() {
    let envelope = Envelope { from: 1, to: None };
    let frame = encode_enveloped(&envelope, &Tiny::Blob(vec![])).expect("encodes");
    assert_eq!(frame[1], 0xFF);
    assert_eq!(frame[1], ADDRESSED_TO_EVERYONE);
    let (decoded, _) = decode_enveloped::<Tiny>(&frame).expect("decodes");
    assert_eq!(decoded, envelope);
}

#[test]
fn the_everyone_value_is_not_a_player_slot() {
    let to_everyone_by_slot = Envelope {
        from: 0,
        to: Some(ADDRESSED_TO_EVERYONE),
    };
    assert_eq!(
        encode_enveloped(&to_everyone_by_slot, &Tiny::Blob(vec![])),
        Err(WireError::OutOfRange { field: "to" })
    );
    let from_everyone = Envelope {
        from: ADDRESSED_TO_EVERYONE,
        to: None,
    };
    assert_eq!(
        encode_enveloped(&from_everyone, &Tiny::Blob(vec![])),
        Err(WireError::OutOfRange { field: "from" })
    );
    let body = encode(&Tiny::Blob(vec![])).expect("encodes");
    let frame = [&[ADDRESSED_TO_EVERYONE, 0][..], &body].concat();
    assert_eq!(
        decode_enveloped::<Tiny>(&frame),
        Err(WireError::OutOfRange { field: "from" })
    );
}

#[test]
fn a_frame_shorter_than_the_envelope_is_truncated() {
    assert_eq!(decode_enveloped::<Tiny>(&[]), Err(WireError::Empty));
    assert_eq!(decode_enveloped::<Tiny>(&[0]), Err(WireError::Truncated));
    assert_eq!(decode_enveloped::<Tiny>(&[0, 1]), Err(WireError::Empty));
}

#[test]
fn the_body_limit_is_still_the_message_limit() {
    assert_eq!(MAX_MESSAGE_BYTES, 512);
    assert_eq!(MAX_FRAME_BYTES, MAX_MESSAGE_BYTES + ENVELOPE_BYTES);
    let envelope = Envelope { from: 0, to: None };
    let largest = Tiny::Blob(vec![0; MAX_MESSAGE_BYTES - 3]);
    let frame = encode_enveloped(&envelope, &largest).expect("encodes");
    assert_eq!(frame.len(), MAX_FRAME_BYTES);
    assert_eq!(decode_enveloped::<Tiny>(&frame), Ok((envelope, largest)));
    let oversize = Tiny::Blob(vec![0; MAX_MESSAGE_BYTES]);
    assert!(matches!(
        encode_enveloped(&envelope, &oversize),
        Err(WireError::TooLarge {
            limit: MAX_MESSAGE_BYTES,
            ..
        })
    ));
    let mut too_long = frame;
    too_long.push(0);
    assert!(matches!(
        decode_enveloped::<Tiny>(&too_long),
        Err(WireError::TooLarge {
            limit: MAX_MESSAGE_BYTES,
            ..
        })
    ));
}

#[test]
fn a_frame_with_bytes_after_the_message_is_read_with_the_extra_counted() {
    let envelope = Envelope {
        from: 1,
        to: Some(0),
    };
    let message = Tiny::Blob(vec![4, 5]);
    let mut frame = encode_enveloped(&envelope, &message).expect("encodes");
    frame.extend_from_slice(&[9, 9]);
    assert_eq!(
        decode_enveloped::<Tiny>(&frame),
        Err(WireError::TrailingBytes { extra: 2 })
    );
    assert_eq!(
        decode_enveloped_ignoring_trailing::<Tiny>(&frame),
        Ok((envelope, message, 2))
    );
}

#[test]
fn an_exact_frame_has_no_extra_bytes_when_read_ignoring_trailing() {
    let envelope = Envelope { from: 2, to: None };
    let message = Tiny::Blob(vec![1]);
    let frame = encode_enveloped(&envelope, &message).expect("encodes");
    assert_eq!(
        decode_enveloped_ignoring_trailing::<Tiny>(&frame),
        Ok((envelope, message, 0))
    );
}

#[test]
fn reading_ignoring_trailing_still_refuses_a_truncated_or_invalid_message() {
    assert_eq!(
        decode_enveloped_ignoring_trailing::<Tiny>(&[]),
        Err(WireError::Empty)
    );
    assert_eq!(
        decode_enveloped_ignoring_trailing::<Tiny>(&[1, 0xFF]),
        Err(WireError::Empty)
    );
    let forbidden = postcard::to_stdvec(&Tiny::Forbidden).expect("serializes");
    let frame = frame_enveloped(&Envelope { from: 1, to: None }, &forbidden).expect("frames");
    assert!(decode_enveloped_ignoring_trailing::<Tiny>(&frame).is_err());
}

#[test]
fn a_framed_body_is_written_as_given() {
    let body = [7, 8, 9];
    let frame = frame_enveloped(
        &Envelope {
            from: 1,
            to: Some(0),
        },
        &body,
    )
    .expect("frames");
    assert_eq!(frame, vec![1, 0, 7, 8, 9]);
    assert_eq!(
        frame_enveloped(
            &Envelope {
                from: 0xFF,
                to: None
            },
            &body
        ),
        Err(WireError::OutOfRange { field: "from" })
    );
}
