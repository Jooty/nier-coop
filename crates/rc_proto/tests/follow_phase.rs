use rc_proto::{FadeSync, FollowPhase, MAX_PHASE_POINT, Message, WireError, decode, encode};

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
        ..rc_proto::SessionOptions::default()
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

#[test]
fn a_fade_message_survives_a_round_trip_and_a_bad_one_is_refused() {
    let fade = FadeSync {
        fading_out: true,
        kind: 2,
        depth: 0x20,
        seconds: 1.0,
    };
    let message = Message::Fade(fade);
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message));
    assert!(
        encode(&Message::Fade(FadeSync {
            seconds: f32::NAN,
            ..fade
        }))
        .is_err()
    );
    assert!(encode(&Message::Fade(FadeSync { kind: 9, ..fade })).is_err());
}

#[test]
fn session_options_default_to_sixty_percent_health_and_ten_percent_damage() {
    let options = rc_proto::SessionOptions::default();
    assert_eq!(options.enemy_health_percent, 60);
    assert_eq!(options.enemy_damage_percent, 10);
}

#[test]
fn session_options_clamp_to_the_host_ranges() {
    let wild = rc_proto::SessionOptions {
        enemy_health_percent: u16::MAX,
        enemy_damage_percent: u16::MAX,
        ..rc_proto::SessionOptions::default()
    };
    let clamped = wild.clamped();
    assert_eq!(clamped.enemy_health_percent, 300);
    assert_eq!(clamped.enemy_damage_percent, 50);
}

#[test]
fn a_cutscene_skip_ask_round_trips() {
    let message = Message::CutsceneSkip(rc_proto::CutsceneSkipAsk);
    assert!(message.is_relayed());
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message));
}
