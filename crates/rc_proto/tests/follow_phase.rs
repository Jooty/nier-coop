use rc_proto::{FollowPhase, MAX_PHASE_POINT, Message, WireError, decode, encode};

#[test]
fn a_follow_phase_message_survives_a_round_trip() {
    let message = Message::FollowPhase(FollowPhase { phase: 5, point: 2 });
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_negative_arrival_point_is_refused() {
    let message = Message::FollowPhase(FollowPhase {
        phase: 5,
        point: -1,
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "point" })
    );
}

#[test]
fn an_arrival_point_past_the_limit_is_refused() {
    let message = Message::FollowPhase(FollowPhase {
        phase: 5,
        point: MAX_PHASE_POINT + 1,
    });
    assert!(message.validate().is_err());
}

#[test]
fn follow_phase_is_relayed_and_bye_is_not() {
    assert!(Message::FollowPhase(FollowPhase { phase: 0, point: 0 }).is_relayed());
    assert!(!Message::Bye.is_relayed());
}

#[test]
fn session_options_survive_a_round_trip_and_are_relayed() {
    let message = Message::SessionOptions(rc_proto::SessionOptions {
        show_names: false,
        show_party_frame: true,
        show_distance: true,
        show_ping: false,
    });
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message.clone()));
    assert!(message.is_relayed());
}

#[test]
fn player_settings_survive_a_round_trip_and_stay_between_a_game_and_its_launcher() {
    let message = Message::Settings(rc_proto::PlayerSettings {
        keep_running: true,
        event_log: false,
    });
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message.clone()));
    assert!(!message.is_relayed());
}
