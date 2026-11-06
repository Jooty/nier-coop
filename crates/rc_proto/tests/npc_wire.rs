use rc_proto::{
    ENEMY_GROUP_COUNT, MAX_MESSAGE_BYTES, MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS,
    MAX_NPC_SNAPSHOT_PARTS, MAX_NPCS_PER_SNAPSHOT, MAX_POSITION_ABS, MAX_YAW_ABS_RADIANS, Message,
    MotionState, NPC_UNGROUPED, NpcId, NpcSnapshot, NpcState, WireError, decode, encode,
};

fn npc(group: u16, ordinal: u16) -> NpcState {
    NpcState {
        id: NpcId {
            group,
            class: 104,
            ordinal,
        },
        position: [10.0, -2.5, 300.0],
        yaw_radians: 0.75,
        motion: Some(MotionState {
            index: 4,
            frame: 12.0,
            speed: 1.0,
            looping: true,
        }),
        visible: true,
    }
}

fn widest_npc() -> NpcState {
    NpcState {
        id: NpcId {
            group: NPC_UNGROUPED,
            class: u16::MAX,
            ordinal: u16::MAX,
        },
        position: [-MAX_POSITION_ABS; 3],
        yaw_radians: -MAX_YAW_ABS_RADIANS,
        motion: Some(MotionState {
            index: u16::MAX,
            frame: MAX_MOTION_FRAME,
            speed: -MAX_MOTION_SPEED_ABS,
            looping: true,
        }),
        visible: false,
    }
}

fn snapshot(npcs: Vec<NpcState>) -> NpcSnapshot {
    NpcSnapshot {
        tick: 9,
        phase: 3,
        part: 0,
        parts: 1,
        total: u16::try_from(npcs.len()).expect("small"),
        npcs,
    }
}

#[test]
fn an_npc_snapshot_survives_a_round_trip() {
    let message = Message::NpcSnapshot(snapshot(vec![npc(5, 0), npc(NPC_UNGROUPED, 1)]));
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_full_snapshot_part_of_the_widest_npcs_fits_one_message() {
    let mut part = snapshot(vec![widest_npc(); MAX_NPCS_PER_SNAPSHOT]);
    part.tick = u32::MAX;
    part.phase = u16::MAX;
    part.part = MAX_NPC_SNAPSHOT_PARTS - 1;
    part.parts = MAX_NPC_SNAPSHOT_PARTS;
    part.total = u16::MAX;
    let bytes = encode(&Message::NpcSnapshot(part)).expect("fits");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
}

#[test]
fn a_snapshot_part_with_too_many_npcs_is_refused() {
    let part = snapshot(vec![npc(1, 0); MAX_NPCS_PER_SNAPSHOT + 1]);
    assert_eq!(
        encode(&Message::NpcSnapshot(part)),
        Err(WireError::OutOfRange {
            field: "npc_snapshot_npcs"
        })
    );
}

#[test]
fn an_npc_part_number_outside_the_part_count_is_refused() {
    for (part, parts) in [(0, 0), (1, 1), (0, MAX_NPC_SNAPSHOT_PARTS + 1)] {
        let bad = NpcSnapshot {
            part,
            parts,
            ..snapshot(Vec::new())
        };
        assert!(encode(&Message::NpcSnapshot(bad)).is_err());
    }
}

#[test]
fn an_npc_with_bad_numbers_is_refused() {
    let bad = [
        NpcState {
            id: NpcId {
                group: ENEMY_GROUP_COUNT,
                class: 1,
                ordinal: 0,
            },
            ..npc(1, 0)
        },
        NpcState {
            position: [f32::NAN, 0.0, 0.0],
            ..npc(1, 0)
        },
        NpcState {
            yaw_radians: f32::INFINITY,
            ..npc(1, 0)
        },
    ];
    for state in bad {
        let message = Message::NpcSnapshot(snapshot(vec![state]));
        assert!(encode(&message).is_err(), "{state:?}");
    }
}

#[test]
fn npc_snapshots_are_relayed_unreliably() {
    let message = Message::NpcSnapshot(snapshot(Vec::new()));
    assert!(message.is_latest_wins());
    assert!(message.is_relayed());
}
