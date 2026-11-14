use automata_game::character::Character;
use automata_game::downed::DownedSignal;
use automata_game::event_follow::EventAskWhat;
use automata_game::exe::ExeVariant;
use automata_game::partner::{RecordedStart, RemoteStateIds};
use automata_game::partner_flight::FlightReport;
use automata_game::shared_defeat::{DefeatNote, DefeatWay, RestorePlace};
use automata_game::story_steps::{StepAnswer, StoryStepAnswer, StoryStepAsk, StoryStepDone};
use automata_proto::*;
use nt_wire::{MemberPing, STORY_MODE, SessionRules};

const GOLDEN_HEX: &str = include_str!("golden_wire.hex");

fn pos() -> [f32; 3] {
    [1.5, -2.0, 3.25]
}

fn place() -> StoryPlace {
    StoryPlace::new(0x100, "00_30_A_Place")
}

fn enemy_placed() -> EnemyId {
    EnemyId::Placed {
        layout: 0x11,
        entry: 0x22,
    }
}

fn enemy_generated() -> EnemyId {
    EnemyId::Generated {
        generator: 0x33,
        serial: 4,
    }
}

fn event() -> EventId {
    EventId {
        kind: 0,
        id: 0x123,
        sub: -1,
    }
}

fn player_state() -> PlayerState {
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
        place: place(),
        playing_event: None,
        flight: FlightReport::default(),
        unattended: false,
        watching: false,
    }
}

fn samples() -> Vec<Message> {
    vec![
        Message::Hello(Hello {
            version: PROTOCOL_VERSION,
            build: BuildGate {
                time_date_stamp: 0x1122_3344,
                size_of_image: 0x0055_6677,
            },
            name: "Nier".to_owned(),
        }),
        Message::Welcome(Welcome {
            version: PROTOCOL_VERSION,
            slot: 1,
            host_name: "Host".to_owned(),
            rules: SessionRules {
                mode: STORY_MODE,
                max_players: 2,
                map: 3,
            },
        }),
        Message::Reject(Reject {
            reason: RejectReason::BuildMismatch,
        }),
        Message::Ping(Ping { nonce: 0xDEAD_BEEF }),
        Message::Pong(Pong { nonce: 0xFEED_FACE }),
        Message::Bye,
        Message::Character(CharacterHello {
            character: Character::TwoB,
            exe: ExeVariant::Main,
        }),
        Message::State(player_state()),
        Message::PathInfo(PathInfo {
            rtt_micros: 42_000,
            direct: true,
        }),
        Message::LobbyReady(LobbyReady {
            ready: true,
            mode: 1,
            slot: Some(2),
        }),
        Message::LobbyStart(LobbyStart { mode: STORY_MODE }),
        Message::LobbyCharacter(LobbyCharacter { index: 2 }),
        Message::Readiness(GameReadiness::InPlay),
        Message::Teleport(TeleportRequest::FollowJump { point: 8 }),
        Message::EnemyRoster(EnemyRosterPart {
            round: 3,
            last: true,
            enemies: vec![EnemySighting {
                id: enemy_placed(),
                object_id: 0x5000,
                health: 120,
                max_health: 200,
            }],
        }),
        Message::EnemyHits(EnemyHits {
            hits: vec![EnemyHit {
                id: enemy_generated(),
                amount: 15,
                max_health: 200,
            }],
        }),
        Message::EnemyDefeated(enemy_placed()),
        Message::EnemySpawned(EnemySpawn {
            generator: 0x33,
            serial: 4,
            set_index: 1,
            object_id: 0x5000,
            level: Some(7),
            uses_the_second_set_values: true,
            object_read_variant: 2,
            set_values: [1, 2, 3, 4],
            position: [1.5, -2.0, 3.25, 1.0],
            rotation: [0.0, 0.5, 0.0, 1.0],
        }),
        Message::EnemyGone(enemy_generated()),
        Message::GeneratorStates(GeneratorStates {
            states: vec![GeneratorState {
                generator: 0x33,
                phase: GeneratorPhase::Spawning,
                spawned: 5,
                counts: None,
            }],
        }),
        Message::LiveGenerators(LiveGenerators {
            route_hash: 0xCAFE_F00D,
            part: 0,
            parts: 1,
            ids: vec![0x33, 0x34],
        }),
        Message::EnemyLead(EnemyLead {
            poses: vec![EnemyPose {
                id: enemy_placed(),
                position: pos(),
                facing: 0.5,
                state: [1, 2, 3, 4],
                starts: 9,
                target: LeadTarget::HostPlayer,
            }],
        }),
        Message::Member(Member {
            slot: 1,
            name: "Partner".to_owned(),
        }),
        Message::MemberLeft { slot: 1 },
        Message::Rules(SessionRules {
            mode: STORY_MODE,
            max_players: 2,
            map: 3,
        }),
        Message::MemberPings(vec![MemberPing {
            slot: 1,
            rtt_ms: 42,
        }]),
        Message::Pickup(ItemPickup {
            item_id: 80,
            count: 2,
            key_item: false,
        }),
        Message::StoryMove(StoryMove {
            number: 2,
            from: place(),
            request: StoryRequest::Full {
                id: 0x40,
                name: "Step".to_owned(),
                params: [1, 2, 3, 4, 5, 6],
            },
        }),
        Message::StoryAsk(StoryAsk {
            number: 3,
            at: place(),
            request: StoryRequest::SubPhase {
                id: 0x41,
                name: "Sub".to_owned(),
            },
        }),
        Message::StoryAskAnswer(StoryAskAnswer {
            number: 3,
            verdict: AskVerdict::RefusedOtherPlace,
        }),
        Message::EventStarted { id: event() },
        Message::EventEnded {
            id: event(),
            skipped: true,
        },
        Message::EventAsk(EventAsk {
            id: event(),
            what: EventAskWhat::Skip,
        }),
        Message::HostSaved { number: 7 },
        Message::SkipAsk { id: event() },
        Message::DefeatNote(DefeatNote::Way {
            way: DefeatWay::RestorePoint,
            restore: Some(RestorePlace {
                place: StoryPlace::new(0x100, "p_a"),
                has_a_point: true,
            }),
        }),
        Message::StoryStepDone(StoryStepDone {
            number: 2,
            id: 0x1234,
            value: -1,
            place: StoryPlace::new(0x100, "A"),
        }),
        Message::StoryStepAsk(StoryStepAsk {
            id: 0x1234,
            value: 3,
            place: StoryPlace::new(0x100, "A"),
        }),
        Message::StoryStepAnswer(StoryStepAnswer {
            id: 0x1234,
            answer: StepAnswer::NotStarted,
        }),
        Message::EventPause {
            id: event(),
            paused: true,
        },
    ]
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn golden_lines() -> Vec<(&'static str, &'static str)> {
    GOLDEN_HEX
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_once(' ').expect("a golden line is `Name hex`"))
        .collect()
}

#[test]
fn the_protocol_number_is_pinned_here_and_nowhere_else() {
    assert_eq!(PROTOCOL_VERSION, 1036);
}

#[test]
fn every_message_variant_keeps_its_encoded_bytes() {
    let golden = golden_lines();
    let samples = samples();
    for (index, message) in samples.iter().enumerate() {
        let name = message.kind_name();
        let actual = hex_of(&encode(message).expect("a sample encodes"));
        let Some((golden_name, golden_hex)) = golden.get(index) else {
            panic!(
                "variant {name} (sample {index}) has no line in golden_wire.hex: `{name} {actual}`"
            );
        };
        assert_eq!(
            *golden_name, name,
            "golden line {index} names another variant"
        );
        assert_eq!(
            *golden_hex, actual,
            "the encoding of variant {name} changed"
        );
    }
    assert_eq!(
        golden.len(),
        samples.len(),
        "golden_wire.hex has lines for variants that have no sample"
    );
}

#[test]
fn samples_are_the_variants_in_declaration_order() {
    for (index, message) in samples().iter().enumerate() {
        let bytes = encode(message).expect("a sample encodes");
        assert_eq!(
            usize::from(bytes[0]),
            index,
            "sample {index} is variant {}, so it is out of declaration order",
            message.kind_name()
        );
    }
}

#[test]
fn every_variant_has_a_sample() {
    let next_index = u8::try_from(samples().len()).expect("fewer than 128 variants");
    let beyond_last = postcard::from_bytes::<Message>(&[next_index]);
    assert!(
        matches!(beyond_last, Err(postcard::Error::SerdeDeCustom)),
        "Message has a variant after sample {next_index}: add its sample and its golden line"
    );
}

#[test]
fn every_golden_sample_decodes_back_to_itself() {
    for message in samples() {
        let bytes = encode(&message).expect("a sample encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
}
