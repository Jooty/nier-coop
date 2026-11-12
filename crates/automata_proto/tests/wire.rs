use automata_game::character::Character;
use automata_game::downed::{DownedPhase, DownedSignal};
use automata_game::exe::ExeVariant;
use automata_game::partner::{BodySnapshot, PodAim, RecordedStart, RemoteStateIds};
use automata_proto::{
    BuildGate, CharacterHello, EnemyHit, EnemyHits, EnemyId, EnemyLead, EnemyPose, EnemyRosterPart,
    EnemySighting, EnemySpawn, GeneratorPhase, GeneratorState, GeneratorStates, Hello, LeadTarget,
    LoadSignal, MAX_MESSAGE_BYTES, MAX_PLAYERS, Message, PROTOCOL_VERSION, PathInfo, Ping,
    PlayerState, Pong, Reject, RejectReason, TeleportRequest, Welcome, WireError, decode, encode,
};
use nc_wire::{
    GameMessage, GameReadiness, LinkPayload, LobbyCharacter, LobbyMessage, LobbyPayload,
    LobbyReady, LobbyStart,
};

const BUILD: BuildGate = BuildGate {
    time_date_stamp: 0x6AC8_9519,
    size_of_image: 0x0100_0000,
};

fn state() -> PlayerState {
    PlayerState {
        sequence: 7,
        held: 0x0000_0101,
        pressed: 0x0000_0100,
        speed: 0.75,
        world_facing: 1.25,
        position: [1.5, -2.0, 300.25],
        body_facing: -0.5,
        state: 12,
        last_start: Some(RecordedStart {
            count: 41,
            ids: RemoteStateIds {
                state: 12,
                variant: 2,
                b: 0xFFFF_FFFF,
                c: 3,
            },
        }),
        health: 4200,
        max_health: 5000,
        free: true,
        phase_id: 0x100,
        paused: false,
        jump_pending: false,
        load: LoadSignal::default(),
        pod_aim: None,
        downed: DownedSignal::default(),
    }
}

fn round_trip(message: Message) {
    let bytes = encode(&message).expect("valid message encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn every_message_kind_survives_a_round_trip() {
    round_trip(Message::Hello(Hello {
        version: PROTOCOL_VERSION,
        build: BUILD,
        name: "Nier".to_owned(),
    }));
    round_trip(Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot: 1,
        host_name: "Host".to_owned(),
    }));
    round_trip(Message::Reject(Reject {
        reason: RejectReason::VersionMismatch,
    }));
    round_trip(Message::Ping(Ping { nonce: 9 }));
    round_trip(Message::Pong(Pong { nonce: 9 }));
    round_trip(Message::Bye);
    for character in Character::ALL {
        for exe in ExeVariant::ALL {
            round_trip(Message::Character(CharacterHello { character, exe }));
        }
    }
    round_trip(Message::State(state()));
    round_trip(Message::PathInfo(PathInfo {
        rtt_micros: 42_000,
        direct: true,
    }));
    for payload in lobby_payloads() {
        round_trip(Message::from_lobby(payload));
    }
    round_trip(Message::Teleport(TeleportRequest::ComeToSender));
    round_trip(Message::Teleport(TeleportRequest::FollowJump { point: 21 }));
}

#[test]
fn a_teleport_request_is_relayed_reliably_and_keeps_its_wire_number() {
    let come = Message::Teleport(TeleportRequest::ComeToSender);
    assert_eq!(come.kind_name(), "Teleport");
    assert!(come.is_relayed());
    assert!(!come.is_latest_wins());
    assert_eq!(GameMessage::ordered_tick(&come), None);
    assert_eq!(encode(&come).expect("encodes"), vec![13, 0]);
    let follow = Message::Teleport(TeleportRequest::FollowJump { point: 8 });
    assert_eq!(encode(&follow).expect("encodes"), vec![13, 1, 8]);
    assert!(come.clone().into_link().is_err());
    assert!(come.into_lobby().is_err());
}

#[test]
fn a_jump_to_a_point_the_game_does_not_name_is_refused() {
    let unknown = Message::Teleport(TeleportRequest::FollowJump { point: 22 });
    assert_eq!(
        encode(&unknown),
        Err(WireError::OutOfRange { field: "point" })
    );
    assert_eq!(
        decode(&[13, 1, 22]),
        Err(WireError::OutOfRange { field: "point" })
    );
}

fn lobby_payloads() -> [LobbyPayload; 4] {
    [
        LobbyPayload::Ready(LobbyReady {
            ready: true,
            mode: 1,
            slot: Some(2),
        }),
        LobbyPayload::Start(LobbyStart { mode: 0 }),
        LobbyPayload::Character(LobbyCharacter { index: 2 }),
        LobbyPayload::GameReadiness(GameReadiness::InPlay),
    ]
}

#[test]
fn lobby_payloads_map_both_ways_and_are_relayed() {
    for payload in lobby_payloads() {
        let message = Message::from_lobby(payload);
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert_eq!(message.into_lobby(), Ok(payload));
    }
    assert!(Message::State(state()).into_lobby().is_err());
}

#[test]
fn a_lobby_message_out_of_range_is_refused() {
    let start = Message::LobbyStart(LobbyStart { mode: 9 });
    assert_eq!(encode(&start), Err(WireError::OutOfRange { field: "mode" }));
}

#[test]
fn the_messages_the_hooks_already_send_keep_their_wire_numbers() {
    let hello = Message::Character(CharacterHello {
        character: Character::TwoB,
        exe: ExeVariant::Main,
    });
    assert_eq!(encode(&hello).expect("encodes")[0], 6);
    assert_eq!(encode(&Message::State(state())).expect("encodes")[0], 7);
    let report = Message::PathInfo(PathInfo {
        rtt_micros: 1,
        direct: true,
    });
    assert_eq!(encode(&report).expect("encodes")[0], 8);
}

#[test]
fn a_state_carries_every_field_of_the_snapshot_both_ways() {
    let snapshot = BodySnapshot {
        held: 3,
        pressed: 1,
        speed: 1.0,
        world_facing: 2.0,
        position: [10.0, 20.0, 30.0],
        body_facing: 1.0,
        state: 0x44,
        last_start: Some(RecordedStart {
            count: 8,
            ids: RemoteStateIds {
                state: 0x44,
                variant: 1,
                b: 2,
                c: 3,
            },
        }),
        health: 900,
        max_health: 1200,
        free: true,
        phase_id: 0x210,
        paused: true,
        jump_pending: true,
        pod_aim: Some(PodAim {
            eye: [12.0, 21.5, 28.0],
            direction: [0.0, -0.6, 0.8],
        }),
        downed: DownedSignal {
            revives: true,
            phase: DownedPhase::Downed,
            times_downed: 3,
            reviving_down: 7,
            reviving_percent: 100,
        },
    };
    let state = PlayerState::from_snapshot(&snapshot, 65_537);
    assert_eq!(state.sequence, 65_537);
    assert_eq!(state.to_snapshot(), snapshot);
    let Ok(Message::State(decoded)) = decode(&encode(&Message::State(state)).expect("encodes"))
    else {
        panic!("expected a state message");
    };
    assert_eq!(decoded.to_snapshot(), snapshot);
}

#[test]
fn the_path_report_is_not_relayed_to_the_other_player() {
    let report = Message::PathInfo(PathInfo {
        rtt_micros: 1,
        direct: false,
    });
    assert!(!report.is_relayed());
    assert!(!report.is_latest_wins());
}

#[test]
fn the_protocol_version_is_in_this_games_range() {
    assert_eq!(PROTOCOL_VERSION, 1017);
    assert_eq!(<Message as GameMessage>::PROTOCOL_VERSION, 1017);
    assert_eq!(<Message as GameMessage>::MAX_PLAYERS, MAX_PLAYERS);
    assert_eq!(MAX_PLAYERS, 2);
}

#[test]
fn only_the_state_message_is_latest_wins_and_ordered() {
    let message = Message::State(state());
    assert!(message.is_latest_wins());
    assert_eq!(GameMessage::ordered_tick(&message), Some(7));
    let hello = Message::Character(CharacterHello {
        character: Character::A2,
        exe: ExeVariant::Main,
    });
    assert!(!hello.is_latest_wins());
    assert_eq!(GameMessage::ordered_tick(&hello), None);
}

#[test]
fn link_payloads_map_both_ways() {
    for payload in [
        LinkPayload::Ping(Ping { nonce: 3 }),
        LinkPayload::Pong(Pong { nonce: 3 }),
        LinkPayload::Bye,
    ] {
        let message = Message::from_link(payload.clone());
        assert_eq!(message.into_link(), Ok(payload));
    }
    let game = Message::State(state());
    assert_eq!(game.clone().into_link(), Err(game));
}

#[test]
fn game_messages_are_relayed_and_link_messages_are_not() {
    assert!(Message::State(state()).is_relayed());
    assert!(
        Message::Character(CharacterHello {
            character: Character::NineS,
            exe: ExeVariant::Main,
        })
        .is_relayed()
    );
    assert!(!Message::Bye.is_relayed());
    assert!(!Message::Ping(Ping { nonce: 1 }).is_relayed());
}

#[test]
fn a_state_message_fits_far_below_the_size_limit() {
    let bytes = encode(&Message::State(state())).expect("encodes");
    assert!(bytes.len() < MAX_MESSAGE_BYTES / 4);
}

#[test]
fn a_non_finite_or_huge_position_is_refused() {
    for bad in [f32::NAN, f32::INFINITY, 2_000_000.0] {
        let mut broken = state();
        broken.position[1] = bad;
        assert_eq!(
            encode(&Message::State(broken)),
            Err(WireError::OutOfRange { field: "position" })
        );
    }
}

#[test]
fn a_bad_facing_or_speed_is_refused() {
    let mut world_facing = state();
    world_facing.world_facing = 99.0;
    assert_eq!(
        encode(&Message::State(world_facing)),
        Err(WireError::OutOfRange {
            field: "world_facing"
        })
    );
    let mut body_facing = state();
    body_facing.body_facing = f32::NAN;
    assert_eq!(
        encode(&Message::State(body_facing)),
        Err(WireError::OutOfRange {
            field: "body_facing"
        })
    );
    let mut speed = state();
    speed.speed = f32::NAN;
    assert_eq!(
        encode(&Message::State(speed)),
        Err(WireError::OutOfRange { field: "speed" })
    );
}

#[test]
fn a_speed_no_stick_can_produce_is_refused() {
    for speed in [-0.5, 1_000.5, 5_000.0, f32::INFINITY] {
        let mut state = state();
        state.speed = speed;
        assert_eq!(
            encode(&Message::State(state)),
            Err(WireError::OutOfRange { field: "speed" })
        );
    }
}

#[test]
fn a_state_id_above_the_highest_the_game_has_is_refused_when_sent() {
    let mut live = state();
    live.state = 0xD4;
    assert_eq!(
        encode(&Message::State(live)),
        Err(WireError::OutOfRange { field: "state" })
    );
    let mut started = state();
    started.last_start = Some(RecordedStart {
        count: 1,
        ids: RemoteStateIds {
            state: 0xD4,
            variant: 0,
            b: 0,
            c: 0,
        },
    });
    assert_eq!(
        encode(&Message::State(started)),
        Err(WireError::OutOfRange {
            field: "last_start"
        })
    );
}

#[test]
fn a_state_id_above_the_highest_the_game_has_is_refused_at_decode() {
    let highest = PlayerState {
        state: 0xD3,
        last_start: None,
        ..state()
    };
    let mut bytes = encode(&Message::State(highest)).expect("the highest id encodes");
    let highest_id_as_sent = [0xD3, 0x01];
    let places: Vec<usize> = bytes
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| *pair == highest_id_as_sent)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(places.len(), 1, "the id must be found exactly once");
    bytes[places[0]] = 0xD4;
    assert_eq!(
        decode(&bytes),
        Err(WireError::OutOfRange { field: "state" })
    );
}

#[test]
fn a_welcome_with_a_slot_beyond_the_player_count_is_refused() {
    let message = Message::Welcome(Welcome {
        version: PROTOCOL_VERSION,
        slot: MAX_PLAYERS,
        host_name: "Host".to_owned(),
    });
    assert_eq!(
        encode(&message),
        Err(WireError::OutOfRange { field: "slot" })
    );
}

#[test]
fn an_empty_or_cut_message_is_refused() {
    assert_eq!(decode(&[]), Err(WireError::Empty));
    let bytes = encode(&Message::State(state())).expect("encodes");
    assert_eq!(decode(&bytes[..bytes.len() - 1]), Err(WireError::Truncated));
}

#[test]
fn bytes_after_a_message_are_refused() {
    let mut bytes = encode(&Message::Bye).expect("encodes");
    bytes.push(0);
    assert_eq!(decode(&bytes), Err(WireError::TrailingBytes { extra: 1 }));
}

#[test]
fn a_state_tells_whether_its_sender_is_free_and_in_which_phase() {
    for (free, phase_id) in [(true, 0x100), (false, 0x210), (false, -1)] {
        let sent = PlayerState {
            free,
            phase_id,
            ..state()
        };
        let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
        else {
            panic!("expected a state message");
        };
        assert_eq!(decoded.free, free);
        assert_eq!(decoded.phase_id, phase_id);
        assert_eq!(decoded.to_snapshot().free, free);
        assert_eq!(decoded.to_snapshot().phase_id, phase_id);
    }
}

#[test]
fn a_state_tells_whether_its_sender_is_paused_or_has_a_jump_pending() {
    for (paused, jump_pending) in [(false, false), (true, false), (false, true), (true, true)] {
        let sent = PlayerState {
            paused,
            jump_pending,
            ..state()
        };
        let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
        else {
            panic!("expected a state message");
        };
        assert_eq!(decoded.paused, paused);
        assert_eq!(decoded.jump_pending, jump_pending);
        assert_eq!(decoded.to_snapshot().paused, paused);
        assert_eq!(decoded.to_snapshot().jump_pending, jump_pending);
    }
}

#[test]
fn a_state_tells_what_its_sender_does_on_its_loading_screen() {
    for load in [
        LoadSignal::default(),
        LoadSignal {
            without_body: true,
            waiting: true,
            release_in_ms: None,
        },
        LoadSignal {
            without_body: true,
            waiting: true,
            release_in_ms: Some(500),
        },
        LoadSignal {
            without_body: false,
            waiting: false,
            release_in_ms: Some(i64::MIN),
        },
        LoadSignal {
            without_body: false,
            waiting: false,
            release_in_ms: Some(-1_200),
        },
    ] {
        let sent = PlayerState { load, ..state() };
        let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
        else {
            panic!("expected a state message");
        };
        assert_eq!(decoded.load, load);
    }
}

#[test]
fn a_state_made_from_a_body_says_nothing_about_a_loading_screen() {
    let made = PlayerState::from_snapshot(&state().to_snapshot(), 9);
    assert_eq!(made.load, LoadSignal::default());
}

#[test]
fn a_state_without_a_body_is_a_valid_message() {
    let from_a_loading_screen = PlayerState {
        load: LoadSignal {
            without_body: true,
            waiting: true,
            release_in_ms: None,
        },
        ..PlayerState::from_snapshot(&BodySnapshot::default(), 1)
    };
    assert_eq!(Message::State(from_a_loading_screen).validate(), Ok(()));
}

#[test]
fn a_state_without_a_pod_aim_is_a_valid_message() {
    let sent = PlayerState {
        pod_aim: None,
        ..state()
    };
    let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
    else {
        panic!("expected a state message");
    };
    assert_eq!(decoded.pod_aim, None);
    assert_eq!(decoded.to_snapshot().pod_aim, None);
}

#[test]
fn a_pod_aim_that_is_not_finite_or_has_no_unit_direction_is_refused() {
    let bad_aims = [
        PodAim {
            eye: [f32::NAN, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
        },
        PodAim {
            eye: [0.0, 2_000_000.0, 0.0],
            direction: [0.0, 0.0, 1.0],
        },
        PodAim {
            eye: [1.0, 2.0, 3.0],
            direction: [0.0, 0.0, 0.0],
        },
        PodAim {
            eye: [1.0, 2.0, 3.0],
            direction: [0.0, f32::INFINITY, 0.0],
        },
        PodAim {
            eye: [1.0, 2.0, 3.0],
            direction: [0.0, 3.0, 4.0],
        },
    ];
    for pod_aim in bad_aims {
        let broken = PlayerState {
            pod_aim: Some(pod_aim),
            ..state()
        };
        assert_eq!(
            encode(&Message::State(broken)),
            Err(WireError::OutOfRange { field: "pod_aim" })
        );
    }
}

#[test]
fn a_state_with_a_pod_aim_still_fits_far_below_the_size_limit() {
    let with_aim = PlayerState {
        pod_aim: Some(PodAim {
            eye: [-1234.5, 67.25, 8901.0],
            direction: [0.6, 0.0, -0.8],
        }),
        ..state()
    };
    let bytes = encode(&Message::State(with_aim)).expect("encodes");
    assert!(bytes.len() < MAX_MESSAGE_BYTES / 4);
}

fn worst_case_spawn() -> EnemySpawn {
    EnemySpawn {
        generator: u32::MAX,
        serial: u32::MAX,
        set_index: 255,
        object_id: 0x2_FFFF,
        level: Some(-32_768),
        uses_the_second_set_values: true,
        object_read_variant: u32::MAX,
        set_values: [i32::MIN; 4],
        position: [-999_999.5, 999_999.5, -999_999.5, 1.0],
        rotation: [-3.1, 3.1, -3.1, 1.0],
    }
}

fn worst_case_enemy(entry: u32) -> EnemySighting {
    EnemySighting {
        id: EnemyId::Placed {
            layout: u32::MAX,
            entry: u32::MAX - entry,
        },
        object_id: 0x2_FFFF,
        health: 100_000_000,
        max_health: 100_000_000,
    }
}

#[test]
fn the_spawn_messages_survive_a_round_trip_and_are_relayed_reliably() {
    let spawned = Message::EnemySpawned(worst_case_spawn());
    let gone = Message::EnemyGone(worst_case_spawn().id());
    let states = Message::GeneratorStates(GeneratorStates {
        states: vec![GeneratorState {
            generator: 0x5A01_77C2,
            phase: GeneratorPhase::Resting,
            spawned: 12,
        }],
    });
    for (message, name, number) in [
        (spawned, "EnemySpawned", 17),
        (gone, "EnemyGone", 18),
        (states, "GeneratorStates", 19),
    ] {
        assert_eq!(message.kind_name(), name);
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert_eq!(GameMessage::ordered_tick(&message), None);
        assert_eq!(encode(&message).expect("encodes")[0], number);
        assert!(message.clone().into_link().is_err());
        round_trip(message);
    }
}

#[test]
fn a_full_list_of_generator_states_fits_the_size_limit_and_a_longer_one_is_refused() {
    let state = GeneratorState {
        generator: u32::MAX,
        phase: GeneratorPhase::Other,
        spawned: u32::MAX,
    };
    let full = Message::GeneratorStates(GeneratorStates {
        states: vec![state; 16],
    });
    assert!(encode(&full).expect("encodes").len() <= MAX_MESSAGE_BYTES);
    assert!(
        encode(&Message::EnemySpawned(worst_case_spawn()))
            .expect("encodes")
            .len()
            <= MAX_MESSAGE_BYTES
    );
    let too_long = Message::GeneratorStates(GeneratorStates {
        states: vec![state; 17],
    });
    assert!(matches!(
        encode(&too_long),
        Err(WireError::OutOfRange { field: "states" })
    ));
}

#[test]
fn a_spawn_that_no_generator_can_have_made_is_refused() {
    let no_generator = EnemySpawn {
        generator: 0,
        ..worst_case_spawn()
    };
    let no_such_set = EnemySpawn {
        set_index: 256,
        ..worst_case_spawn()
    };
    let nowhere = EnemySpawn {
        position: [f32::NAN, 0.0, 0.0, 1.0],
        ..worst_case_spawn()
    };
    let no_such_level = EnemySpawn {
        level: Some(40_000),
        ..worst_case_spawn()
    };
    for spawn in [no_generator, no_such_set, nowhere, no_such_level] {
        assert!(matches!(
            encode(&Message::EnemySpawned(spawn)),
            Err(WireError::OutOfRange { field: "spawn" })
        ));
    }
}

#[test]
fn the_enemy_messages_survive_a_round_trip_and_are_relayed_reliably() {
    let roster = Message::EnemyRoster(EnemyRosterPart {
        round: 3,
        last: true,
        enemies: vec![worst_case_enemy(0), worst_case_enemy(1)],
    });
    let hits = Message::EnemyHits(EnemyHits {
        hits: vec![EnemyHit {
            id: EnemyId::Placed {
                layout: 7,
                entry: 9,
            },
            amount: 40,
            max_health: 200,
        }],
    });
    let defeated = Message::EnemyDefeated(EnemyId::Generated {
        generator: 0x5A01_77C2,
        serial: 9,
    });
    for (message, name, number) in [
        (roster, "EnemyRoster", 14),
        (hits, "EnemyHits", 15),
        (defeated, "EnemyDefeated", 16),
    ] {
        assert_eq!(message.kind_name(), name);
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert_eq!(GameMessage::ordered_tick(&message), None);
        assert_eq!(encode(&message).expect("encodes")[0], number);
        assert!(message.clone().into_link().is_err());
        round_trip(message);
    }
}

#[test]
fn a_full_roster_part_and_a_full_hit_list_fit_the_size_limit() {
    let roster = Message::EnemyRoster(EnemyRosterPart {
        round: u32::MAX,
        last: true,
        enemies: (0..16).map(worst_case_enemy).collect(),
    });
    assert!(encode(&roster).expect("encodes").len() <= MAX_MESSAGE_BYTES);
    let hits = Message::EnemyHits(EnemyHits {
        hits: (0..16)
            .map(|entry| EnemyHit {
                id: worst_case_enemy(entry).id,
                amount: 100_000_000,
                max_health: 100_000_000,
            })
            .collect(),
    });
    assert!(encode(&hits).expect("encodes").len() <= MAX_MESSAGE_BYTES);
}

#[test]
fn an_enemy_message_past_its_limits_is_refused() {
    let roster = Message::EnemyRoster(EnemyRosterPart {
        round: 1,
        last: true,
        enemies: (0..17).map(worst_case_enemy).collect(),
    });
    assert!(matches!(
        encode(&roster),
        Err(WireError::OutOfRange { field: "enemies" })
    ));
    let hits = Message::EnemyHits(EnemyHits {
        hits: vec![EnemyHit {
            id: worst_case_enemy(0).id,
            amount: 1,
            max_health: 0,
        }],
    });
    assert!(matches!(
        encode(&hits),
        Err(WireError::OutOfRange { field: "hits" })
    ));
}

#[test]
fn the_downed_signal_crosses_the_wire_and_a_percent_above_100_is_refused() {
    let signal = DownedSignal {
        revives: true,
        phase: DownedPhase::Defeated,
        times_downed: 255,
        reviving_down: 2,
        reviving_percent: 99,
    };
    let sent = PlayerState {
        downed: signal,
        ..state()
    };
    let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
    else {
        panic!("expected a state message");
    };
    assert_eq!(decoded.downed, signal);
    let too_far = PlayerState {
        downed: DownedSignal {
            reviving_percent: 101,
            ..signal
        },
        ..state()
    };
    assert_eq!(
        Message::State(too_far).validate(),
        Err(WireError::OutOfRange { field: "downed" })
    );
}

fn worst_case_pose(entry: u32) -> EnemyPose {
    EnemyPose {
        id: EnemyId::Placed {
            layout: u32::MAX,
            entry: u32::MAX - entry,
        },
        position: [-999_999.0, 999_999.0, -999_999.0],
        facing: -15.0,
        state: [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        starts: u32::MAX,
        target: LeadTarget::SomethingElse,
    }
}

#[test]
fn the_enemy_lead_is_relayed_reliably_after_the_spawn_messages() {
    let lead = Message::EnemyLead(EnemyLead {
        poses: vec![worst_case_pose(0)],
    });
    assert_eq!(lead.kind_name(), "EnemyLead");
    assert!(lead.is_relayed());
    assert!(!lead.is_latest_wins());
    assert_eq!(GameMessage::ordered_tick(&lead), None);
    assert_eq!(encode(&lead).expect("encodes")[0], 20);
    assert!(lead.clone().into_link().is_err());
    round_trip(lead);
}

#[test]
fn a_full_enemy_lead_fits_the_size_limit_and_a_longer_one_is_refused() {
    let full = Message::EnemyLead(EnemyLead {
        poses: (0..8).map(worst_case_pose).collect(),
    });
    assert!(encode(&full).expect("encodes").len() <= MAX_MESSAGE_BYTES);
    let longer = Message::EnemyLead(EnemyLead {
        poses: (0..9).map(worst_case_pose).collect(),
    });
    assert!(longer.validate().is_err());
}
