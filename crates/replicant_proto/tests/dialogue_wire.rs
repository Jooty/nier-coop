use replicant_proto::{
    DialogueLine, DialogueUpdate, MAX_DIALOGUE_LINE_INDEX, MAX_TALK_ID, Message, WireError, decode,
    encode,
};

#[test]
fn a_dialogue_line_round_trips_and_is_relayed() {
    for update in [
        DialogueUpdate::Line(DialogueLine {
            talk_id: 1234,
            line: 3,
        }),
        DialogueUpdate::Ended,
        DialogueUpdate::StoryLine(DialogueLine {
            talk_id: 1226,
            line: 1,
        }),
    ] {
        let message = Message::Dialogue(update);
        assert!(message.is_relayed());
        assert_eq!(decode(&encode(&message).unwrap()).unwrap(), message);
    }
}

#[test]
fn a_talk_id_or_line_past_the_game_table_is_refused() {
    let line =
        |talk_id, line| Message::Dialogue(DialogueUpdate::Line(DialogueLine { talk_id, line }));
    assert_eq!(
        line(MAX_TALK_ID, MAX_DIALOGUE_LINE_INDEX).validate(),
        Ok(())
    );
    assert_eq!(
        line(MAX_TALK_ID + 1, 0).validate(),
        Err(WireError::OutOfRange { field: "talk_id" })
    );
    assert_eq!(
        line(0, MAX_DIALOGUE_LINE_INDEX + 1).validate(),
        Err(WireError::OutOfRange { field: "line" })
    );
}

#[test]
fn a_story_line_past_the_game_table_is_refused() {
    let story = |talk_id, line| {
        Message::Dialogue(DialogueUpdate::StoryLine(DialogueLine { talk_id, line }))
    };
    assert_eq!(
        story(MAX_TALK_ID, MAX_DIALOGUE_LINE_INDEX).validate(),
        Ok(())
    );
    assert_eq!(
        story(MAX_TALK_ID + 1, 0).validate(),
        Err(WireError::OutOfRange { field: "talk_id" })
    );
    assert_eq!(
        story(0, MAX_DIALOGUE_LINE_INDEX + 1).validate(),
        Err(WireError::OutOfRange { field: "line" })
    );
}
