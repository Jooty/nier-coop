use replicant_game::downed::{
    BLEED_OUT_SECONDS, DOWNED_HEALTH, DownedEffect, DownedRules, GAME_OVER_FLOW_WINDOW_MS,
    GUEST_WAITS_FOR_HOST_GAME_OVER_SECONDS, HostGameOverGate, RewindDrop, StepInput,
};
use replicant_game::story_progress::{
    PLAYER_NAME_BYTES, SAVE_FILE_BYTES, SLOT_BYTES, SLOTS_FILE_OFFSET, SlotStory, StoryArrays,
    StoryMirror, StoryRule, StorySource, apply_story_rule, milestone_ids,
};
use replicant_proto::{CombatEvent, StoryArray, StoryArrayPart, StoryChange};

const FULL_HEALTH: i32 = 400;
const TICK_SECONDS: f32 = 0.4;

fn step(health: i32) -> StepInput {
    StepInput {
        dt_seconds: TICK_SECONDS,
        session_active: true,
        local_health: health,
        lethal_hit_seen: health == DOWNED_HEALTH,
        partner_distance: None,
    }
}

fn guest_with_checkpoint() -> DownedRules {
    let mut rules = DownedRules::new();
    rules.set_guest(true);
    rules.step(step(FULL_HEALTH));
    assert_eq!(
        rules.on_partner_event(CombatEvent::HostCheckpoint),
        vec![DownedEffect::TakeBackup]
    );
    rules.backup_taken();
    rules
}

fn both_down(rules: &mut DownedRules) -> Vec<DownedEffect> {
    rules.on_partner_event(CombatEvent::Downed);
    rules.step(step(DOWNED_HEALTH))
}

fn sends(effects: &[DownedEffect]) -> Vec<CombatEvent> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            DownedEffect::Send(event) => Some(*event),
            _ => None,
        })
        .collect()
}

#[test]
fn the_guest_rewinds_on_the_hosts_game_over_without_killing_itself() {
    let mut rules = guest_with_checkpoint();
    let effects = both_down(&mut rules);
    assert!(!effects.contains(&DownedEffect::ForceKill));
    assert!(sends(&effects).contains(&CombatEvent::GameOver));
    let effects = rules.on_partner_event(CombatEvent::HostGameOver);
    assert_eq!(effects, vec![DownedEffect::Rewind]);
    rules.rewind_done();
    assert!(!rules.is_game_over());
    assert!(!rules.is_rewinding());
}

#[test]
fn a_game_over_from_the_host_does_not_kill_the_guest() {
    let mut rules = guest_with_checkpoint();
    let effects = rules.on_partner_event(CombatEvent::GameOver);
    assert!(effects.is_empty());
    assert!(rules.is_game_over());
}

#[test]
fn the_host_still_kills_itself_on_a_game_over() {
    let mut rules = DownedRules::new();
    rules.step(step(FULL_HEALTH));
    assert_eq!(
        rules.on_partner_event(CombatEvent::GameOver),
        vec![DownedEffect::ForceKill]
    );
}

#[test]
fn a_guest_only_bleed_out_never_kills_the_guest_and_sends_no_game_over() {
    let mut rules = guest_with_checkpoint();
    let mut all = rules.step(step(DOWNED_HEALTH));
    let mut waited = 0.0;
    while waited < BLEED_OUT_SECONDS + TICK_SECONDS {
        all.extend(rules.step(step(DOWNED_HEALTH)));
        waited += TICK_SECONDS;
    }
    assert!(!all.contains(&DownedEffect::ForceKill));
    assert!(!rules.is_game_over());
    let events = sends(&all);
    assert!(!events.contains(&CombatEvent::GameOver));
    assert!(!events.contains(&CombatEvent::HostGameOver));
    assert!(!events.contains(&CombatEvent::HostCheckpoint));
}

#[test]
fn a_host_game_over_before_any_checkpoint_changes_nothing() {
    let mut rules = DownedRules::new();
    rules.set_guest(true);
    rules.step(step(FULL_HEALTH));
    assert_eq!(
        rules.on_partner_event(CombatEvent::HostGameOver),
        vec![DownedEffect::DropRewind(RewindDrop::NoCheckpoint)]
    );
}

#[test]
fn a_second_host_game_over_during_one_rewind_is_ignored() {
    let mut rules = guest_with_checkpoint();
    assert_eq!(
        rules.on_partner_event(CombatEvent::HostGameOver),
        vec![DownedEffect::Rewind]
    );
    assert_eq!(
        rules.on_partner_event(CombatEvent::HostGameOver),
        vec![DownedEffect::DropRewind(RewindDrop::AlreadyRewinding)]
    );
    rules.rewind_done();
    assert_eq!(
        rules.on_partner_event(CombatEvent::HostGameOver),
        vec![DownedEffect::Rewind]
    );
}

#[test]
fn a_rewind_that_never_got_to_play_falls_back_to_the_local_kill_when_both_were_down() {
    let mut rules = guest_with_checkpoint();
    both_down(&mut rules);
    rules.on_partner_event(CombatEvent::HostGameOver);
    assert_eq!(rules.rewind_dropped(), vec![DownedEffect::ForceKill]);
    assert!(!rules.is_rewinding());
}

#[test]
fn the_guest_kills_itself_after_ten_seconds_without_the_hosts_game_over() {
    let mut rules = guest_with_checkpoint();
    both_down(&mut rules);
    let mut kills = 0;
    let mut waited = 0.0;
    while waited < GUEST_WAITS_FOR_HOST_GAME_OVER_SECONDS - 2.0 * TICK_SECONDS {
        kills += rules
            .step(step(DOWNED_HEALTH))
            .iter()
            .filter(|effect| **effect == DownedEffect::ForceKill)
            .count();
        waited += TICK_SECONDS;
    }
    assert_eq!(kills, 0);
    for _ in 0..10 {
        kills += rules
            .step(step(DOWNED_HEALTH))
            .iter()
            .filter(|effect| **effect == DownedEffect::ForceKill)
            .count();
    }
    assert_eq!(kills, 1);
}

#[test]
fn the_hosts_game_over_stops_the_local_kill_fallback() {
    let mut rules = guest_with_checkpoint();
    both_down(&mut rules);
    rules.on_partner_event(CombatEvent::HostGameOver);
    for _ in 0..40 {
        let effects = rules.step(step(DOWNED_HEALTH));
        assert!(!effects.contains(&DownedEffect::ForceKill));
    }
}

#[test]
fn ending_the_session_forgets_the_checkpoint() {
    let mut rules = guest_with_checkpoint();
    let mut input = step(FULL_HEALTH);
    input.session_active = false;
    rules.step(input);
    rules.set_guest(true);
    rules.step(step(FULL_HEALTH));
    assert_eq!(
        rules.on_partner_event(CombatEvent::HostGameOver),
        vec![DownedEffect::DropRewind(RewindDrop::NoCheckpoint)]
    );
}

#[test]
fn getting_healthy_again_after_a_game_over_keeps_the_checkpoint() {
    let mut rules = guest_with_checkpoint();
    both_down(&mut rules);
    rules.step(step(FULL_HEALTH));
    assert_eq!(
        rules.on_partner_event(CombatEvent::HostGameOver),
        vec![DownedEffect::Rewind]
    );
}

#[test]
fn the_host_sends_no_game_over_for_a_restore_outside_the_game_over_path() {
    let mut gate = HostGameOverGate::new();
    assert!(!gate.take(1_000));
}

#[test]
fn both_down_and_the_script_message_give_exactly_one_host_game_over() {
    let mut gate = HostGameOverGate::new();
    gate.arm(1_000);
    gate.arm(2_000);
    assert!(gate.take(3_000));
    assert!(!gate.take(3_100));
}

#[test]
fn a_game_over_flow_that_never_restored_expires() {
    let mut gate = HostGameOverGate::new();
    gate.arm(1_000);
    assert!(!gate.take(1_000 + GAME_OVER_FLOW_WINDOW_MS + 1));
}

fn arrays_with(changes: &[StoryChange]) -> StoryArrays {
    let mut arrays = StoryArrays::default();
    for &change in changes {
        arrays.apply(change);
    }
    arrays
}

fn whole_parts(arrays: &StoryArrays) -> Vec<StoryArrayPart> {
    StoryArray::ALL
        .into_iter()
        .map(|array| StoryArrayPart {
            array,
            bytes: arrays.bytes(array).to_vec(),
        })
        .collect()
}

fn flag(id: u16, on: bool) -> StoryChange {
    StoryChange::Flag {
        array: StoryArray::GameFlags,
        id,
        on,
    }
}

#[test]
fn after_a_rewind_the_guests_own_drift_waits_but_the_hosts_rewound_story_applies_at_once() {
    let mut mirror = StoryMirror::default();
    let host = arrays_with(&[flag(9, true)]);
    for part in whole_parts(&host) {
        mirror.receive_part(&part);
    }
    assert!(mirror.plan(&host, 0).fixes.is_empty());
    mirror.hold_own_drift_until(10_000);
    let mut live = host.clone();
    live.apply(flag(400, true));
    assert!(mirror.plan(&live, 5_000).fixes.is_empty());
    mirror.receive_changes(&[flag(9, false)]);
    let plan = mirror.plan(&live, 6_000);
    assert_eq!(plan.fixes, vec![flag(9, false)]);
    live.apply(flag(9, false));
    let later = mirror.plan(&live, 10_000 + 60_000);
    assert_eq!(later.fixes, vec![flag(400, false)]);
}

fn slot_with_milestones(milestones: usize) -> SlotStory {
    let mut slot = vec![0_u8; SLOT_BYTES];
    slot[4..10].copy_from_slice(b"A_HOST");
    slot[0x24] = 13;
    let changes: Vec<StoryChange> = milestone_ids(StoryArray::GameFlags)
        .iter()
        .take(milestones)
        .map(|&id| flag(id, true))
        .collect();
    arrays_with(&changes).write_to_block(&mut slot);
    SlotStory::read(&slot).expect("a whole slot")
}

fn write_slot(file: &mut [u8], story: &SlotStory) {
    let slot = &mut file[SLOTS_FILE_OFFSET..SLOTS_FILE_OFFSET + SLOT_BYTES];
    slot[..4].copy_from_slice(&200_u32.to_le_bytes());
    let mut name = [0_u8; PLAYER_NAME_BYTES];
    name[..4].copy_from_slice(b"NIER");
    slot[replicant_game::story_progress::PLAYER_NAME_BLOCK_OFFSET
        ..replicant_game::story_progress::PLAYER_NAME_BLOCK_OFFSET + PLAYER_NAME_BYTES]
        .copy_from_slice(&name);
    story.write(slot);
    replicant_game::story_progress::fix_slot_checksum(slot);
}

#[test]
fn the_save_rule_never_writes_below_the_high_mark_after_a_rewind() {
    let mut name = [0_u8; PLAYER_NAME_BYTES];
    name[..4].copy_from_slice(b"NIER");
    let mut rule = StoryRule::new(slot_with_milestones(0), name, None);
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    let empty_disk = file.clone();
    write_slot(&mut file, &slot_with_milestones(60));
    apply_story_rule(&mut file, Some(&empty_disk), &mut rule);
    let on_disk = file.clone();
    write_slot(&mut file, &slot_with_milestones(20));
    let outcomes = apply_story_rule(&mut file, Some(&on_disk), &mut rule);
    let source = outcomes.iter().find_map(|outcome| match outcome {
        replicant_game::story_progress::SlotOutcome::Written { source, .. } => Some(*source),
        _ => None,
    });
    assert_eq!(source, Some(StorySource::SessionBest));
    let saved = SlotStory::read(&file[SLOTS_FILE_OFFSET..SLOTS_FILE_OFFSET + SLOT_BYTES])
        .expect("a whole slot");
    assert_eq!(saved, slot_with_milestones(60));
}
