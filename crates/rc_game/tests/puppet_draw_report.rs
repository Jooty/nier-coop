use rc_game::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_DRAW_MASK_OFFSET, ACTOR_DRAWN_FLAG_OFFSET,
    ACTOR_LIFE_STATE_OFFSET, ACTOR_MODEL_PACK_OFFSET, ACTOR_SCRIPT_GROUP_OFFSET,
    PLAYABLE_ACTOR_X_OFFSET, PLAYABLE_ACTOR_Y_OFFSET, PLAYABLE_ACTOR_Z_OFFSET,
};
use rc_game::puppet::{distance_between, read_actor_draw_report};

use crate::fake_memory::FakeMemory;

const ACTOR: u64 = 0x2_0000_0000;

#[test]
fn the_report_carries_the_position_and_the_draw_bytes() {
    let mut memory = FakeMemory::new();
    memory.put(ACTOR + PLAYABLE_ACTOR_X_OFFSET, &1.5_f32.to_le_bytes());
    memory.put(ACTOR + PLAYABLE_ACTOR_Y_OFFSET, &2.5_f32.to_le_bytes());
    memory.put(ACTOR + PLAYABLE_ACTOR_Z_OFFSET, &(-3.5_f32).to_le_bytes());
    memory.put(ACTOR + ACTOR_DRAWN_FLAG_OFFSET, &[1]);
    memory.put(ACTOR + ACTOR_DRAW_MASK_OFFSET, &[0xF]);
    memory.put(ACTOR + ACTOR_CLASS_ID_OFFSET, &0xAA_i32.to_le_bytes());
    memory.put(ACTOR + ACTOR_SCRIPT_GROUP_OFFSET, &7_i32.to_le_bytes());
    memory.put(ACTOR + ACTOR_LIFE_STATE_OFFSET, &0_i32.to_le_bytes());
    memory.put(ACTOR + ACTOR_MODEL_PACK_OFFSET, &0x1234_u64.to_le_bytes());
    let report = read_actor_draw_report(&memory, ACTOR);
    let report = report.expect("every field is mapped");
    assert_eq!(report.position, [1.5, 2.5, -3.5]);
    assert_eq!((report.drawn_flag, report.draw_mask), (1, 0xF));
    assert_eq!(
        (report.class_id, report.script_group, report.model_pack),
        (0xAA, 7, 0x1234)
    );
}

#[test]
fn the_distance_is_straight_line() {
    assert!((distance_between([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]) - 5.0).abs() < 1e-5);
}
