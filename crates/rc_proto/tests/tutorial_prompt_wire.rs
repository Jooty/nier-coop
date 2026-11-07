use rc_proto::{
    FIRST_TUTORIAL_ID, LAST_TUTORIAL_ID, Message, TutorialPrompt, WireError, decode, encode,
};

#[test]
fn a_tutorial_prompt_round_trips_and_is_relayed() {
    let message = Message::TutorialPrompt(TutorialPrompt { id: 2004 });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn the_first_and_last_tutorial_ids_are_allowed() {
    for id in [FIRST_TUTORIAL_ID, LAST_TUTORIAL_ID] {
        assert_eq!(TutorialPrompt { id }.validate(), Ok(()));
    }
}

#[test]
fn an_id_outside_the_tutorial_range_is_refused() {
    for id in [0, FIRST_TUTORIAL_ID - 1, LAST_TUTORIAL_ID + 1, u16::MAX] {
        assert_eq!(
            Message::TutorialPrompt(TutorialPrompt { id }).validate(),
            Err(WireError::OutOfRange { field: "id" })
        );
    }
}
