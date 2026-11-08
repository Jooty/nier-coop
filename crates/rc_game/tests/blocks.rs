use rc_game::GameMemory;
use rc_game::blocks::{
    BLOCK_REMOTE_OWNERSHIP_MS, BLOCK_SETTLE_MS, BLOCK_VTABLE_RVA, BlockOutbox, BlockRoster,
    BlockSighting, StateTrail, block_is_live, cell_of, changed_words, read_block_position,
    write_block_position,
};
use rc_game::offsets::ACTOR_TRANSFORM_BLOCK_OFFSET;

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const BLOCK_ACTOR: u64 = 0x5000_0000;

fn sighting(actor: u64, x: f32, z: f32) -> BlockSighting {
    BlockSighting {
        actor,
        position: [x, 150.5, z],
    }
}

fn memory_with_block_at(position: [f32; 3]) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(
        BLOCK_ACTOR,
        &(FAKE_MODULE_BASE + u64::from(BLOCK_VTABLE_RVA)).to_le_bytes(),
    );
    let mut transform = [0_u8; 48];
    for (row, axis) in position.iter().enumerate() {
        transform[row * 16 + 12..row * 16 + 16].copy_from_slice(&axis.to_le_bytes());
    }
    memory.put(BLOCK_ACTOR + ACTOR_TRANSFORM_BLOCK_OFFSET, &transform);
    memory
}

#[test]
fn block_ids_follow_the_first_seen_cell_not_the_actor_address() {
    let mut here = BlockRoster::default();
    let mut there = BlockRoster::default();
    here.observe(
        3,
        &[sighting(0x900, -22.0, 18.0), sighting(0x100, -30.0, 5.0)],
    );
    there.observe(
        3,
        &[sighting(0x200, -30.0, 5.0), sighting(0x800, -22.0, 18.0)],
    );
    assert_eq!(here.id_of(0x100), Some(0));
    assert_eq!(here.id_of(0x900), Some(1));
    assert_eq!(there.id_of(0x200), Some(0));
    assert_eq!(there.id_of(0x800), Some(1));
}

#[test]
fn a_block_keeps_its_id_after_it_is_pushed_and_a_new_one_gets_the_next() {
    let mut roster = BlockRoster::default();
    roster.observe(3, &[sighting(0x100, -22.0, 18.0)]);
    let added = roster.observe(
        3,
        &[sighting(0x100, -24.0, 18.0), sighting(0x300, 0.0, 0.0)],
    );
    assert_eq!(roster.id_of(0x100), Some(0));
    assert_eq!(roster.id_of(0x300), Some(1));
    assert_eq!(added.len(), 1);
    assert_eq!(roster.blocks()[0].start_cell, [-22, 151, 18]);
}

#[test]
fn the_roster_starts_over_when_the_map_changes_or_every_block_is_gone() {
    let mut roster = BlockRoster::default();
    roster.observe(3, &[sighting(0x100, 1.0, 1.0), sighting(0x200, 2.0, 2.0)]);
    roster.observe(4, &[sighting(0x200, 2.0, 2.0)]);
    assert_eq!(roster.id_of(0x200), Some(0));
    roster.observe(4, &[sighting(0x700, 9.0, 9.0)]);
    assert_eq!(roster.id_of(0x700), Some(0));
    assert_eq!(roster.id_of(0x200), None);
}

#[test]
fn the_pusher_sends_each_time_the_block_changes_cell_and_not_on_the_first_look() {
    let mut outbox = BlockOutbox::default();
    let at = |x: f32| move |_: u16| Some([x, 150.5, 18.0]);
    assert_eq!(outbox.observe(0, Some(0), at(-22.0)), None);
    assert_eq!(outbox.observe(100, Some(0), at(-22.2)), None);
    assert_eq!(
        outbox.observe(200, Some(0), at(-23.0)),
        Some((0, [-23.0, 150.5, 18.0]))
    );
    assert_eq!(outbox.observe(300, Some(0), at(-23.1)), None);
}

#[test]
fn after_the_release_the_settled_position_is_sent_once_and_then_nothing() {
    let mut outbox = BlockOutbox::default();
    let at = |x: f32| move |_: u16| Some([x, 150.5, 18.0]);
    outbox.observe(0, Some(0), at(-22.0));
    outbox.observe(100, Some(0), at(-23.0));
    assert_eq!(
        outbox.observe(200, None, at(-23.4)),
        Some((0, [-23.4, 150.5, 18.0]))
    );
    assert_eq!(outbox.observe(250, None, at(-23.8)), None);
    assert_eq!(
        outbox.observe(400, None, at(-24.0)),
        Some((0, [-24.0, 150.5, 18.0]))
    );
    assert_eq!(outbox.observe(200 + BLOCK_SETTLE_MS, None, at(-24.0)), None);
    assert_eq!(outbox.tracking(), None);
    assert_eq!(outbox.observe(9_000, None, at(-30.0)), None);
}

#[test]
fn a_block_the_partner_just_moved_is_not_sent_back() {
    let mut outbox = BlockOutbox::default();
    let at = |x: f32| move |_: u16| Some([x, 150.5, 18.0]);
    outbox.note_remote_move(1_000, 0);
    assert_eq!(outbox.observe(1_100, Some(0), at(-22.0)), None);
    assert_eq!(outbox.observe(1_200, Some(0), at(-23.0)), None);
    assert_eq!(outbox.tracking(), None);
    let later = 1_000 + BLOCK_REMOTE_OWNERSHIP_MS;
    assert_eq!(outbox.observe(later, Some(0), at(-23.0)), None);
    assert_eq!(
        outbox.observe(later + 100, Some(0), at(-24.0)),
        Some((0, [-24.0, 150.5, 18.0]))
    );
}

#[test]
fn a_block_the_partner_moves_while_tracked_stops_being_ours() {
    let mut outbox = BlockOutbox::default();
    outbox.observe(0, Some(2), |_| Some([0.0; 3]));
    assert_eq!(outbox.tracking(), Some(2));
    outbox.note_remote_move(10, 2);
    assert_eq!(outbox.tracking(), None);
}

#[test]
fn the_state_diff_lists_changed_words_outside_the_transform() {
    let before = vec![0_u8; 0x400];
    let mut after = before.clone();
    after[0x120] = 1;
    after[ACTOR_TRANSFORM_BLOCK_OFFSET as usize + 12] = 9;
    let changes = changed_words(&before, &after);
    assert_eq!(changes.len(), 1);
    assert_eq!(
        (changes[0].offset, changes[0].before, changes[0].after),
        (0x120, 0, 1)
    );
}

#[test]
fn a_state_trail_reports_each_stage_once() {
    let mut trail = StateTrail::new("idle", vec![0; 16]);
    let mut grabbed = vec![0_u8; 16];
    grabbed[4] = 7;
    let diff = trail
        .advance("grabbed", grabbed.clone())
        .expect("a new stage");
    assert_eq!((diff.from_stage, diff.to_stage), ("idle", "grabbed"));
    assert_eq!(diff.changes.len(), 1);
    assert!(trail.advance("grabbed", grabbed).is_none());
    assert_eq!(trail.stage(), "grabbed");
}

#[test]
fn a_block_position_is_written_and_read_back_through_the_matrix_translation() {
    let mut memory = memory_with_block_at([-22.0, 150.5, 18.0]);
    assert!(block_is_live(&memory, BLOCK_ACTOR));
    write_block_position(&mut memory, BLOCK_ACTOR, [-24.0, 150.5, 19.0]).unwrap();
    assert_eq!(
        read_block_position(&memory, BLOCK_ACTOR),
        Some([-24.0, 150.5, 19.0])
    );
}

#[test]
fn an_actor_with_another_vtable_is_not_a_block() {
    let mut memory = memory_with_block_at([0.0; 3]);
    memory.put(BLOCK_ACTOR, &(FAKE_MODULE_BASE + 0x1234).to_le_bytes());
    assert!(!block_is_live(&memory, BLOCK_ACTOR));
    assert!(!block_is_live(&memory, 0));
    assert_eq!(memory.module_base(), FAKE_MODULE_BASE);
}

#[test]
fn a_cell_is_the_position_rounded_to_whole_units() {
    assert_eq!(cell_of([-22.0, 150.5, 18.4]), [-22, 151, 18]);
}
