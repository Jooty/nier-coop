use rc_proto::{Message, Pong};

#[test]
fn a_message_kind_is_its_variant_name_without_the_payload() {
    assert_eq!(Message::Pong(Pong { nonce: 7 }).kind_name(), "Pong");
    assert_eq!(Message::Bye.kind_name(), "Bye");
}
