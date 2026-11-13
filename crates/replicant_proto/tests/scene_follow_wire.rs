use replicant_proto::{
    MAX_MESSAGE_BYTES, MAX_SCENE_CUT_NAME_BYTES, MAX_SCENE_FOLLOW_CALLBACKS, Message,
    PROTOCOL_VERSION, SceneCut, SceneFollow, WireError, decode, encode,
};

fn follow() -> SceneFollow {
    SceneFollow {
        phase: 25,
        callbacks: vec!["EID_0050_a0030g_End".to_owned()],
        cut: None,
    }
}

fn entrance_follow() -> SceneFollow {
    SceneFollow {
        phase: 25,
        callbacks: vec!["EID_1410_a0060_Start".to_owned()],
        cut: Some(SceneCut {
            event: "EID_1410_a0060".to_owned(),
            audio: "MA_EID_1410_a0060".to_owned(),
        }),
    }
}

#[test]
fn a_scene_follow_round_trips_fits_a_message_and_is_relayed() {
    let message = Message::SceneFollow(follow());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
    assert_eq!(PROTOCOL_VERSION, 69);
}

#[test]
fn a_scene_follow_carrying_the_hosts_loaded_cut_round_trips() {
    let message = Message::SceneFollow(entrance_follow());
    assert_eq!(message.validate(), Ok(()));
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_cut_with_no_audio_is_valid() {
    let mut no_audio = entrance_follow();
    if let Some(cut) = no_audio.cut.as_mut() {
        cut.audio.clear();
    }
    assert_eq!(no_audio.validate(), Ok(()));
}

#[test]
fn a_bad_cut_name_is_refused() {
    let refused = Err(WireError::OutOfRange {
        field: "scene follow cut name",
    });
    let with_cut = |event: &str, audio: &str| SceneFollow {
        cut: Some(SceneCut {
            event: event.to_owned(),
            audio: audio.to_owned(),
        }),
        ..follow()
    };
    assert_eq!(with_cut("", "MA_EID_1410_a0060").validate(), refused);
    assert_eq!(with_cut("two words", "").validate(), refused);
    assert_eq!(
        with_cut(&"E".repeat(MAX_SCENE_CUT_NAME_BYTES + 1), "").validate(),
        refused
    );
    assert_eq!(
        with_cut("EID_1410_a0060", &"M".repeat(MAX_SCENE_CUT_NAME_BYTES + 1)).validate(),
        refused
    );
}

#[test]
fn a_bad_scene_follow_is_refused() {
    let mut many = follow();
    many.callbacks = vec!["A".to_owned(); MAX_SCENE_FOLLOW_CALLBACKS + 1];
    let mut empty_name = follow();
    empty_name.callbacks = vec![String::new()];
    let mut spaced = follow();
    spaced.callbacks = vec!["two words".to_owned()];
    let mut long = follow();
    long.callbacks = vec!["x".repeat(64)];
    assert_eq!(
        many.validate(),
        Err(WireError::OutOfRange {
            field: "scene follow callbacks"
        })
    );
    for bad in [empty_name, spaced, long] {
        assert_eq!(
            bad.validate(),
            Err(WireError::OutOfRange {
                field: "scene follow callback name"
            })
        );
    }
}
