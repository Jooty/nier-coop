use rc_proto::{
    MAX_MESSAGE_BYTES, MAX_MUSIC_FADE_SECONDS, MAX_MUSIC_POSITION_MS, Message, MusicState,
    WireError, decode, encode,
};

fn playing() -> MusicState {
    MusicState {
        track: 4321,
        position_ms: 61_250,
        track_volume: 1.0,
        master_volume: 0.75,
        layer_volumes: [1.0, 0.0, 0.5, 1.0],
        stopping: false,
        stop_fade_seconds: 0.0,
    }
}

#[test]
fn a_music_state_survives_a_round_trip_and_fits_one_message() {
    for state in [playing(), MusicState::silent()] {
        let message = Message::Music(state);
        let bytes = encode(&message).expect("valid state encodes");
        assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn a_music_state_is_relayed_between_the_games() {
    assert!(Message::Music(playing()).is_relayed());
}

#[test]
fn nonsense_music_states_are_refused() {
    let bad_track = MusicState {
        track: -2,
        ..playing()
    };
    let bad_position = MusicState {
        position_ms: MAX_MUSIC_POSITION_MS + 1,
        ..playing()
    };
    let bad_volume = MusicState {
        master_volume: f32::NAN,
        ..playing()
    };
    let bad_layer = MusicState {
        layer_volumes: [0.0, 0.0, f32::INFINITY, 0.0],
        ..playing()
    };
    let bad_fade = MusicState {
        stop_fade_seconds: MAX_MUSIC_FADE_SECONDS + 1.0,
        ..playing()
    };
    for state in [bad_track, bad_position, bad_volume, bad_layer, bad_fade] {
        assert!(matches!(
            state.validate(),
            Err(WireError::OutOfRange { .. })
        ));
        assert!(encode(&Message::Music(state)).is_err());
    }
}
