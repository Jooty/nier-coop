use nc_wire::{
    GameMessage, Hello, LinkPayload, MAX_MESSAGE_BYTES, Reject, RejectReason, WireError, decode,
    encode,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum Tiny {
    Link(Reject),
    Blob(Vec<u8>),
    Forbidden,
}

impl GameMessage for Tiny {
    const PROTOCOL_VERSION: u16 = 1;
    const MAX_PLAYERS: u8 = 2;

    fn from_link(payload: LinkPayload) -> Self {
        match payload {
            LinkPayload::Reject(reject) => Tiny::Link(reject),
            _ => Tiny::Forbidden,
        }
    }

    fn into_link(self) -> Result<LinkPayload, Self> {
        match self {
            Tiny::Link(reject) => Ok(LinkPayload::Reject(reject)),
            other => Err(other),
        }
    }

    fn is_relayed(&self) -> bool {
        matches!(self, Tiny::Blob(_))
    }

    fn is_latest_wins(&self) -> bool {
        false
    }

    fn ordered_tick(&self) -> Option<u32> {
        None
    }

    fn validate(&self) -> Result<(), WireError> {
        match self {
            Tiny::Forbidden => Err(WireError::OutOfRange { field: "kind" }),
            _ => Ok(()),
        }
    }
}

#[test]
fn a_message_round_trips() {
    let message = Tiny::Blob(vec![1, 2, 3]);
    let bytes = encode(&message).expect("encodes");
    assert_eq!(decode::<Tiny>(&bytes), Ok(message));
}

#[test]
fn an_oversize_message_is_refused_both_ways() {
    let message = Tiny::Blob(vec![0; MAX_MESSAGE_BYTES]);
    assert!(matches!(encode(&message), Err(WireError::TooLarge { .. })));
    assert!(matches!(
        decode::<Tiny>(&vec![0; MAX_MESSAGE_BYTES + 1]),
        Err(WireError::TooLarge { .. })
    ));
}

#[test]
fn empty_trailing_and_invalid_input_is_refused() {
    assert_eq!(decode::<Tiny>(&[]), Err(WireError::Empty));
    let mut bytes = encode(&Tiny::Blob(vec![9])).expect("encodes");
    bytes.push(0);
    assert_eq!(
        decode::<Tiny>(&bytes),
        Err(WireError::TrailingBytes { extra: 1 })
    );
    let forbidden = postcard::to_stdvec(&Tiny::Forbidden).expect("serializes");
    assert!(decode::<Tiny>(&forbidden).is_err());
    assert!(encode(&Tiny::Forbidden).is_err());
}

#[test]
fn link_payloads_keep_their_encoded_shape() {
    let reject = Reject {
        reason: RejectReason::BuildMismatch,
    };
    assert_eq!(postcard::to_stdvec(&reject).expect("serializes"), vec![1]);
    let hello = Hello {
        version: 300,
        build: nc_wire::BuildGate {
            time_date_stamp: 1,
            size_of_image: 2,
        },
        name: "A".to_owned(),
    };
    assert_eq!(
        postcard::to_stdvec(&hello).expect("serializes"),
        vec![0xAC, 0x02, 1, 2, 1, b'A']
    );
}
