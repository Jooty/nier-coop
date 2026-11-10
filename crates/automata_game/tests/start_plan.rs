use automata_game::character::Character;
use automata_game::start::{
    ChapterRecord, StartCall, buddy_flag, build_plan, hero_flag, hero_mask_has, route_flag,
};

fn record(group: u8, route: i32) -> ChapterRecord {
    ChapterRecord {
        chapter: 6,
        group,
        route,
        hero_mask: 1,
        phase_id: 0x321,
        phase_name: "made_up".to_owned(),
    }
}

#[test]
fn each_character_has_its_hero_and_buddy_flag() {
    assert_eq!(hero_flag(Character::TwoB), "PL/2B");
    assert_eq!(hero_flag(Character::NineS), "PL/9S");
    assert_eq!(hero_flag(Character::A2), "PL/A2");
    assert_eq!(buddy_flag(Character::TwoB), "Buddy/2B");
    assert_eq!(buddy_flag(Character::NineS), "Buddy/9S");
    assert_eq!(buddy_flag(Character::A2), "Buddy/A2");
}

#[test]
fn the_route_field_picks_the_route_flag() {
    assert_eq!(route_flag(0, Character::TwoB), "Route/A");
    assert_eq!(route_flag(1, Character::TwoB), "Route/B");
    assert_eq!(route_flag(3, Character::TwoB), "Route/C");
    assert_eq!(route_flag(-1, Character::TwoB), "Route/A");
}

#[test]
fn route_two_follows_the_hero() {
    assert_eq!(route_flag(2, Character::TwoB), "Route/A");
    assert_eq!(route_flag(2, Character::NineS), "Route/B");
}

#[test]
fn the_hero_mask_has_one_bit_per_character() {
    assert!(hero_mask_has(0b001, Character::TwoB));
    assert!(hero_mask_has(0b010, Character::NineS));
    assert!(hero_mask_has(0b100, Character::A2));
    assert!(!hero_mask_has(0b001, Character::NineS));
}

#[test]
fn the_plan_sets_the_hero_the_one_buddy_and_the_route_before_anything_else() {
    let plan = build_plan(&record(2, 1), Character::NineS, Character::A2);
    assert_eq!(
        plan.before_phase_start[..4],
        [
            StartCall::WriteHeroModelId(0x10200),
            StartCall::SetFlag("PL/9S".to_owned()),
            StartCall::SetFlag("Buddy/A2".to_owned()),
            StartCall::SetFlag("Route/B".to_owned()),
        ]
    );
    let buddy_flags = plan
        .before_phase_start
        .iter()
        .filter(|call| matches!(call, StartCall::SetFlag(name) if name.starts_with("Buddy/")))
        .count();
    assert_eq!(buddy_flags, 1);
}

#[test]
fn the_four_low_clear_bits_are_cleared_in_order_and_the_mark_is_written_last() {
    let plan = build_plan(&record(2, 0), Character::TwoB, Character::NineS);
    let tail = &plan.before_phase_start[4..];
    assert_eq!(
        tail,
        [
            StartCall::ClearFlagBit(0),
            StartCall::ClearFlagBit(1),
            StartCall::ClearFlagBit(2),
            StartCall::ClearFlagBit(3),
            StartCall::WriteStartedFromChapterSelect,
        ]
    );
}

#[test]
fn the_marker_call_runs_only_for_group_one() {
    let first = build_plan(&record(1, 0), Character::TwoB, Character::NineS);
    assert!(first.before_phase_start.contains(&StartCall::RunMarkerCall));
    let later = build_plan(&record(2, 0), Character::TwoB, Character::NineS);
    assert!(!later.before_phase_start.contains(&StartCall::RunMarkerCall));
}

#[test]
fn the_phase_start_uses_the_chapter_number_plus_one() {
    let plan = build_plan(&record(2, 0), Character::TwoB, Character::NineS);
    assert_eq!(plan.phase_start.phase_id, 0x321);
    assert_eq!(plan.phase_start.phase_name, "made_up");
    assert_eq!(plan.phase_start.chapter_filter, 7);
}

#[test]
fn the_bookkeeping_writes_come_after_the_phase_start() {
    let plan = build_plan(&record(2, 0), Character::TwoB, Character::NineS);
    assert_eq!(
        plan.after_phase_start,
        [
            StartCall::ClearTopBitOfUiBookkeepingWord,
            StartCall::WriteUiBookkeepingFlag
        ]
    );
}

#[test]
fn every_call_describes_itself_for_the_log() {
    let plan = build_plan(&record(1, 0), Character::A2, Character::TwoB);
    for call in plan
        .before_phase_start
        .iter()
        .chain(&plan.after_phase_start)
    {
        assert!(!call.describe().is_empty());
    }
}
