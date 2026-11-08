use rc_proto::{MAX_VIEW_CHAIN_BYTES, Message, ViewState, decode, encode};

#[test]
fn a_view_state_round_trips_and_is_relayed() {
    let message = Message::ViewState(ViewState {
        chain: "of_NierHouse".to_owned(),
        inside: true,
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_view_state_with_an_empty_or_long_or_spaced_chain_is_refused() {
    for chain in [
        String::new(),
        "x".repeat(MAX_VIEW_CHAIN_BYTES + 1),
        "of House".to_owned(),
    ] {
        let message = Message::ViewState(ViewState {
            chain,
            inside: false,
        });
        assert!(message.validate().is_err());
    }
}
