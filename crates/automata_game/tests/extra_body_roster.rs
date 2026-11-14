use automata_game::character::Character;
use automata_game::mode::SessionMode;
use automata_game::partner::{
    ExtraBodyRoster, ExtraBodyStep, OtherPlayer, RosterReadings, SlotStep,
};

const PHASE: i32 = 0x100;
const OTHER_PHASE: i32 = 0x200;
const FRESH_AGE_MS: u64 = 30;
const SILENT_AGE_MS: u64 = 60_000;

fn player(slot: u8, character: Character) -> OtherPlayer {
    OtherPlayer {
        slot,
        character: Some(character),
        in_play: true,
        newest_state_age_ms: Some(FRESH_AGE_MS),
    }
}

fn readings(mode: SessionMode, others: &[OtherPlayer]) -> RosterReadings<'_> {
    RosterReadings {
        mode,
        scene_in_play: true,
        phase_id: PHASE,
        others,
    }
}

fn step_of(steps: &[SlotStep], slot: u8) -> Option<ExtraBodyStep> {
    steps
        .iter()
        .find(|step| step.slot == slot)
        .map(|step| step.step)
}

fn four_players() -> [OtherPlayer; 3] {
    [
        player(1, Character::NineS),
        player(2, Character::A2),
        player(3, Character::TwoB),
    ]
}

fn roster_with_bodies_for(others: &[OtherPlayer]) -> ExtraBodyRoster {
    let mut roster = ExtraBodyRoster::default();
    let steps = roster.steps(&readings(SessionMode::FreeRoam, others), |_| true);
    for step in steps {
        if let ExtraBodyStep::Spawn(character) = step.step {
            roster.made(step.slot, character, 0x1000 + u32::from(step.slot), PHASE);
        }
    }
    roster
}

#[test]
fn campaign_never_makes_an_extra_body() {
    let roster = ExtraBodyRoster::default();
    let others = four_players();
    assert!(
        roster
            .steps(&readings(SessionMode::Campaign, &others), |_| true)
            .is_empty()
    );
    assert!(
        roster
            .steps(&readings(SessionMode::Campaign, &others[..1]), |_| false)
            .is_empty()
    );
}

#[test]
fn free_roam_leaves_the_lowest_slot_to_the_partner_machine() {
    let roster = ExtraBodyRoster::default();
    let others = [player(5, Character::A2), player(2, Character::NineS)];
    let steps = roster.steps(&readings(SessionMode::FreeRoam, &others), |_| true);
    assert_eq!(step_of(&steps, 2), None);
    assert_eq!(
        step_of(&steps, 5),
        Some(ExtraBodyStep::Spawn(Character::A2))
    );
    let alone = [player(2, Character::NineS)];
    assert!(
        roster
            .steps(&readings(SessionMode::FreeRoam, &alone), |_| true)
            .is_empty()
    );
}

#[test]
fn each_further_player_gets_one_body() {
    let others = four_players();
    let roster = roster_with_bodies_for(&others);
    assert_eq!(roster.len(), 2);
    let steps = roster.steps(&readings(SessionMode::FreeRoam, &others), |_| true);
    assert_eq!(step_of(&steps, 2), Some(ExtraBodyStep::Drive(0x1002)));
    assert_eq!(step_of(&steps, 3), Some(ExtraBodyStep::Drive(0x1003)));
    assert_eq!(steps.len(), 2);
    let empty = ExtraBodyRoster::default();
    let unloaded = empty.steps(&readings(SessionMode::FreeRoam, &others), |_| false);
    assert_eq!(
        step_of(&unloaded, 2),
        Some(ExtraBodyStep::RequestData(Character::A2))
    );
}

#[test]
fn a_leaving_player_removes_only_its_body() {
    let others = four_players();
    let mut roster = roster_with_bodies_for(&others);
    let without_slot_two = [others[0], others[2]];
    let steps = roster.steps(&readings(SessionMode::FreeRoam, &without_slot_two), |_| {
        true
    });
    assert_eq!(step_of(&steps, 2), Some(ExtraBodyStep::Remove(0x1002)));
    assert_eq!(step_of(&steps, 3), Some(ExtraBodyStep::Drive(0x1003)));
    roster.gone(2);
    let mut quiet = others;
    quiet[2].newest_state_age_ms = Some(SILENT_AGE_MS);
    let steps = roster.steps(&readings(SessionMode::FreeRoam, &quiet), |_| true);
    assert_eq!(step_of(&steps, 3), Some(ExtraBodyStep::Remove(0x1003)));
    assert_eq!(
        step_of(&steps, 2),
        Some(ExtraBodyStep::Spawn(Character::A2))
    );
}

#[test]
fn a_load_forgets_every_extra_body_and_they_are_made_again() {
    let others = four_players();
    let mut roster = roster_with_bodies_for(&others);
    let mut loading = readings(SessionMode::FreeRoam, &others);
    loading.scene_in_play = false;
    let steps = roster.steps(&loading, |_| true);
    assert_eq!(step_of(&steps, 2), Some(ExtraBodyStep::Forget(0x1002)));
    assert_eq!(step_of(&steps, 3), Some(ExtraBodyStep::Forget(0x1003)));
    let mut next_scene = readings(SessionMode::FreeRoam, &others);
    next_scene.phase_id = OTHER_PHASE;
    let steps = roster.steps(&next_scene, |_| true);
    assert_eq!(step_of(&steps, 2), Some(ExtraBodyStep::Forget(0x1002)));
    roster.gone(2);
    roster.gone(3);
    assert!(roster.is_empty());
    let steps = roster.steps(&next_scene, |_| true);
    assert_eq!(
        step_of(&steps, 2),
        Some(ExtraBodyStep::Spawn(Character::A2))
    );
    assert_eq!(
        step_of(&steps, 3),
        Some(ExtraBodyStep::Spawn(Character::TwoB))
    );
}

#[test]
fn two_players_may_wear_one_character_in_free_roam() {
    let others = [
        player(1, Character::TwoB),
        player(2, Character::TwoB),
        player(3, Character::TwoB),
    ];
    let roster = roster_with_bodies_for(&others);
    assert_eq!(roster.len(), 2);
    assert_eq!(roster.handle_of(2), Some(0x1002));
    assert_eq!(roster.handle_of(3), Some(0x1003));
}
