use replicant_proto::{
    MAX_MESSAGE_BYTES, MAX_STORY_CHANGES_PER_MESSAGE, MAX_STORY_FLAG_ID, MAX_TALK_CHOICE_INDEX,
    MAX_TALK_ID, Message, StoryArray, StoryChange, StoryWrite, TalkChoice, TalkChoiceKind,
    WireError, decode, encode,
};

fn choice(kind: TalkChoiceKind) -> TalkChoice {
    TalkChoice {
        talk_id: 1234,
        line: 2,
        kind,
        index: 1,
        target: 1300,
    }
}

#[test]
fn a_talk_choice_round_trips_and_is_relayed() {
    for kind in [TalkChoiceKind::TalkList, TalkChoiceKind::MenuList] {
        let message = Message::TalkChoice(choice(kind));
        assert!(message.is_relayed());
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    }
}

#[test]
fn a_talk_choice_past_the_game_tables_is_refused() {
    let with = |change: fn(&mut TalkChoice)| {
        let mut value = choice(TalkChoiceKind::TalkList);
        change(&mut value);
        Message::TalkChoice(value).validate()
    };
    assert_eq!(
        with(|value| value.talk_id = MAX_TALK_ID + 1),
        Err(WireError::OutOfRange { field: "talk_id" })
    );
    assert_eq!(
        with(|value| value.index = MAX_TALK_CHOICE_INDEX + 1),
        Err(WireError::OutOfRange { field: "index" })
    );
    assert_eq!(
        with(|value| value.target = MAX_TALK_ID + 1),
        Err(WireError::OutOfRange { field: "target" })
    );
    let menu_value = Message::TalkChoice(TalkChoice {
        target: u32::MAX,
        ..choice(TalkChoiceKind::MenuList)
    });
    assert_eq!(menu_value.validate(), Ok(()));
}

#[test]
fn a_full_story_write_fits_one_message_and_an_empty_one_is_refused() {
    let widest = StoryChange::Flag {
        array: StoryArray::SnowFlags,
        id: MAX_STORY_FLAG_ID,
        on: true,
    };
    let full = Message::StoryWrite(StoryWrite {
        changes: vec![widest; MAX_STORY_CHANGES_PER_MESSAGE],
    });
    assert!(full.is_relayed());
    assert_eq!(full.validate(), Ok(()));
    let bytes = encode(&full).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES, "{} bytes", bytes.len());
    let empty = Message::StoryWrite(StoryWrite { changes: vec![] });
    assert_eq!(
        empty.validate(),
        Err(WireError::OutOfRange { field: "changes" })
    );
}
