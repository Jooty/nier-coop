use replicant_proto::{Message, decode, encode};

#[test]
fn whether_the_partner_has_a_book_survives_a_round_trip() {
    for has_book in [false, true] {
        let message = Message::PartnerBook(has_book);
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn the_book_presence_is_relayed_and_valid() {
    let message = Message::PartnerBook(true);
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
}
