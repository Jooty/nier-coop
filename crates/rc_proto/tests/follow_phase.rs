use rc_proto::{FadeSync, FollowPhase, MAX_PHASE_POINT, Message, WireError, decode, encode};

#[test]
fn a_follow_phase_message_survives_a_round_trip() {
    let message = Message::FollowPhase(FollowPhase::at(5, 2));
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_negative_arrival_point_is_refused() {
    let message = Message::FollowPhase(FollowPhase::at(5, -1));
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "point" })
    );
}

#[test]
fn an_arrival_point_past_the_limit_is_refused() {
    let message = Message::FollowPhase(FollowPhase::at(5, MAX_PHASE_POINT + 1));
    assert!(message.validate().is_err());
}

#[test]
fn follow_phase_is_relayed_and_bye_is_not() {
    assert!(Message::FollowPhase(FollowPhase::at(0, 0)).is_relayed());
    assert!(!Message::Bye.is_relayed());
}

#[test]
fn the_hosts_side_view_and_last_normal_spot_survive_a_round_trip() {
    let message = Message::FollowPhase(FollowPhase {
        in_side_view: true,
        last_normal_view: Some([1.5, -2.0, 30.25]),
        ..FollowPhase::at(5, 2)
    });
    assert!(message.validate().is_ok());
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_garbage_last_normal_spot_is_refused() {
    for axis in [f32::NAN, f32::INFINITY, 2_000_000.0] {
        let message = Message::FollowPhase(FollowPhase {
            last_normal_view: Some([0.0, axis, 0.0]),
            ..FollowPhase::at(5, 2)
        });
        assert_eq!(
            message.validate(),
            Err(WireError::OutOfRange {
                field: "last normal view"
            })
        );
    }
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

#[test]
fn lobby_messages_round_trip_are_relayed_and_refuse_bad_values() {
    let ready = Message::LobbyReady(rc_proto::LobbyReady {
        ready: true,
        mode: rc_proto::LOBBY_MODE_CONTINUE,
        slot: Some(3),
    });
    let start = Message::LobbyStart(rc_proto::LobbyStart {
        mode: rc_proto::LOBBY_MODE_NEW_GAME,
    });
    for message in [ready, start] {
        assert!(message.is_relayed());
        let bytes = encode(&message).expect("valid message encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
    let bad_slot = Message::LobbyReady(rc_proto::LobbyReady {
        ready: true,
        mode: 0,
        slot: Some(rc_proto::MAX_LOBBY_SLOT + 1),
    });
    assert!(encode(&bad_slot).is_err());
    assert!(encode(&Message::LobbyStart(rc_proto::LobbyStart { mode: 9 })).is_err());
}

#[test]
fn the_guests_lobby_pick_round_trips_and_is_relayed() {
    for body in [
        rc_proto::PartnerBody::Nier,
        rc_proto::PartnerBody::PrologueCharacter,
        rc_proto::PartnerBody::SeedKaineE,
    ] {
        let message = Message::LobbyBody(rc_proto::LobbyBody { body });
        assert!(message.is_relayed());
        assert!(message.validate().is_ok());
        let bytes = encode(&message).expect("valid message encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn the_quit_request_round_trips_and_stays_between_launcher_and_game() {
    let message = Message::QuitGame;
    assert!(!message.is_relayed());
    assert!(message.validate().is_ok());
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message));
}
