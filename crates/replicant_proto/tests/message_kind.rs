use replicant_proto::{Message, Pong};

#[test]
fn a_message_kind_is_its_variant_name_without_the_payload() {
    assert_eq!(Message::Pong(Pong { nonce: 7 }).kind_name(), "Pong");
    assert_eq!(Message::Bye.kind_name(), "Bye");
}

const SINGLE_BYTE_VARIANT_LIMIT: u8 = 128;

fn zeroed_message_of_variant(index: u8) -> Option<Message> {
    let mut bytes = vec![0_u8; 256];
    bytes[0] = index;
    postcard::take_from_bytes::<Message>(&bytes)
        .ok()
        .map(|(message, _)| message)
}

#[test]
fn every_message_variant_has_its_own_name_matching_the_variant() {
    let messages: Vec<Message> = (0..SINGLE_BYTE_VARIANT_LIMIT)
        .map_while(zeroed_message_of_variant)
        .collect();
    let decoded = messages.len();
    let first_unused = u8::try_from(decoded).expect("fewer variants than the limit");
    assert!(
        (first_unused..SINGLE_BYTE_VARIANT_LIMIT)
            .all(|index| zeroed_message_of_variant(index).is_none()),
        "a variant after index {decoded} decodes, so one in between does not"
    );
    let mut seen = std::collections::HashSet::new();
    for message in &messages {
        let debug = format!("{message:?}");
        let variant: String = debug.chars().take_while(|c| c.is_alphanumeric()).collect();
        assert_eq!(message.kind_name(), variant);
        assert!(seen.insert(message.kind_name()), "{variant} named twice");
    }
}
