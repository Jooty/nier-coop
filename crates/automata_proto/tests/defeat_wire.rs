use automata_game::load_gate::StoryPlace;
use automata_game::shared_defeat::{DefeatNote, DefeatWay, GameOverChoice, RestorePlace};
use automata_proto::{Message, WireError, decode, encode};
use nt_wire::GameMessage;

const DEFEAT_NOTE_VARIANT_TAG: u8 = 35;

fn a_way() -> DefeatNote {
    DefeatNote::Way {
        way: DefeatWay::RestorePoint,
        restore: Some(RestorePlace {
            place: StoryPlace::new(0x100, "p_a"),
            has_a_point: true,
        }),
    }
}

#[test]
fn the_defeat_note_comes_after_every_existing_variant() {
    for note in [
        a_way(),
        DefeatNote::Way {
            way: DefeatWay::Title,
            restore: None,
        },
        DefeatNote::MenuChoice(GameOverChoice::Retry),
        DefeatNote::MenuChoice(GameOverChoice::Title),
    ] {
        let message = Message::DefeatNote(note);
        let bytes = encode(&message).expect("encodes");
        assert_eq!(bytes[0], DEFEAT_NOTE_VARIANT_TAG);
        assert_eq!(message.kind_name(), "DefeatNote");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn the_defeat_note_is_relayed_reliable_and_not_latest_wins() {
    let message = Message::DefeatNote(DefeatNote::MenuChoice(GameOverChoice::Title));
    assert!(message.is_relayed());
    assert!(!message.is_latest_wins());
    assert!(!GameMessage::forwarded_to_other_guests(&message));
}

#[test]
fn a_defeat_note_with_a_bad_place_name_is_refused() {
    let DefeatNote::Way { way, restore } = a_way() else {
        panic!("a way note");
    };
    let mut restore = restore.expect("a place");
    restore.place.name_bytes[20] = b'z';
    let message = Message::DefeatNote(DefeatNote::Way {
        way,
        restore: Some(restore),
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange {
            field: "defeat_note"
        })
    );
    assert_eq!(Message::DefeatNote(a_way()).validate(), Ok(()));
}
