use rc_proto::{
    MAX_MESSAGE_BYTES, MAX_STORY_CHANGES_PER_MESSAGE, MAX_STORY_FLAG_ID, Message,
    STORY_ARRAY_BYTES, StoryArray, StoryArrayPart, StoryChange, StoryChanges, StoryDigest,
    StoryRequest, decode, encode,
};

fn widest_flag_change() -> StoryChange {
    StoryChange::Flag {
        array: StoryArray::SnowFlags,
        id: MAX_STORY_FLAG_ID,
        on: true,
    }
}

#[test]
fn story_messages_survive_a_round_trip_and_are_relayed() {
    for message in [
        Message::StoryRequest(StoryRequest),
        Message::StoryPart(StoryArrayPart {
            array: StoryArray::GameCounters,
            bytes: vec![0xFF; STORY_ARRAY_BYTES],
        }),
        Message::StoryChanges(StoryChanges {
            changes: vec![
                widest_flag_change(),
                StoryChange::Counter {
                    array: StoryArray::SnowCounters,
                    id: 255,
                    value: 7,
                },
            ],
        }),
        Message::StoryDigest(StoryDigest { hash: u64::MAX }),
    ] {
        let bytes = encode(&message).expect("valid story message encodes");
        assert!(bytes.len() <= MAX_MESSAGE_BYTES, "{} bytes", bytes.len());
        assert_eq!(decode(&bytes), Ok(message.clone()));
        assert!(message.is_relayed());
    }
}

#[test]
fn a_full_batch_of_the_widest_changes_fits_one_message() {
    let message = Message::StoryChanges(StoryChanges {
        changes: vec![widest_flag_change(); MAX_STORY_CHANGES_PER_MESSAGE],
    });
    let bytes = encode(&message).expect("full batch encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES, "{} bytes", bytes.len());
}

#[test]
fn nonsense_story_messages_are_refused() {
    let short_part = Message::StoryPart(StoryArrayPart {
        array: StoryArray::GameFlags,
        bytes: vec![0; STORY_ARRAY_BYTES - 1],
    });
    let flag_past_the_set = StoryChange::Flag {
        array: StoryArray::GameFlags,
        id: MAX_STORY_FLAG_ID + 1,
        on: true,
    };
    let flag_in_a_counter_array = StoryChange::Flag {
        array: StoryArray::GameCounters,
        id: 1,
        on: true,
    };
    let counter_in_a_flag_array = StoryChange::Counter {
        array: StoryArray::SnowFlags,
        id: 1,
        value: 1,
    };
    let mut refused = vec![
        short_part,
        Message::StoryChanges(StoryChanges { changes: vec![] }),
        Message::StoryChanges(StoryChanges {
            changes: vec![widest_flag_change(); MAX_STORY_CHANGES_PER_MESSAGE + 1],
        }),
    ];
    for change in [
        flag_past_the_set,
        flag_in_a_counter_array,
        counter_in_a_flag_array,
    ] {
        refused.push(Message::StoryChanges(StoryChanges {
            changes: vec![change],
        }));
    }
    for message in refused {
        assert!(encode(&message).is_err(), "{message:?}");
    }
}
