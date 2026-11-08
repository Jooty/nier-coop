use rc_game::PhaseEntry;
use rc_game::story_jump::{
    JumpStep, LEVEL_CHECK_SETTLE_MS, LevelCheck, LevelWatcher, StoryJumpRefusal, StoryJumpView,
    check_level, check_story_jump, destination, jump_waits_for_pause_menu, level_check_is_due,
    loadout_player_level, loadout_steps,
};
use rc_proto::StoryJumpPoint;

fn ready_host() -> StoryJumpView {
    StoryJumpView {
        host: true,
        map_manager_active: true,
        event_scene: false,
        talk_window_open: false,
        ui_dialog_state: 0,
        pause_menu_open: false,
    }
}

#[test]
fn a_host_in_the_field_may_jump() {
    assert_eq!(check_story_jump(&ready_host()), Ok(()));
}

#[test]
fn a_guest_is_refused() {
    let view = StoryJumpView {
        host: false,
        ..ready_host()
    };
    assert_eq!(check_story_jump(&view), Err(StoryJumpRefusal::NotHost));
}

#[test]
fn a_host_outside_the_field_is_refused() {
    let loading = StoryJumpView {
        map_manager_active: false,
        ..ready_host()
    };
    assert_eq!(
        check_story_jump(&loading),
        Err(StoryJumpRefusal::NotInTheField)
    );
}

#[test]
fn an_event_scene_refuses_the_jump() {
    let scene = StoryJumpView {
        event_scene: true,
        ..ready_host()
    };
    assert_eq!(check_story_jump(&scene), Err(StoryJumpRefusal::EventScene));
}

#[test]
fn an_open_talk_refuses_the_jump() {
    let talk = StoryJumpView {
        talk_window_open: true,
        ..ready_host()
    };
    assert_eq!(check_story_jump(&talk), Err(StoryJumpRefusal::TalkOpen));
}

#[test]
fn an_open_dialog_refuses_the_jump_and_names_its_state() {
    let dialog = StoryJumpView {
        ui_dialog_state: 4,
        ..ready_host()
    };
    let refusal = check_story_jump(&dialog).unwrap_err();
    assert_eq!(refusal, StoryJumpRefusal::DialogOpen(4));
    assert!(refusal.to_string().contains("0x4"));
}

#[test]
fn the_pause_menu_does_not_refuse_the_jump_but_makes_it_wait() {
    let paused = StoryJumpView {
        pause_menu_open: true,
        ..ready_host()
    };
    assert_eq!(check_story_jump(&paused), Ok(()));
    assert!(jump_waits_for_pause_menu(&paused));
    assert!(!jump_waits_for_pause_menu(&ready_host()));
}

#[test]
fn after_the_prologue_sets_the_end_flags_and_the_loadout() {
    let steps = loadout_steps(StoryJumpPoint::AfterPrologue);
    for expected in [
        JumpStep::SetFlag { id: 71 },
        JumpStep::SetFlag { id: 153 },
        JumpStep::SetItem { id: 1000, count: 1 },
        JumpStep::EquipWeapon { id: 1000 },
        JumpStep::SetPlayerLevel { level: 0 },
        JumpStep::MpFullMode { on: false },
    ] {
        assert!(steps.contains(&expected), "{expected:?}");
    }
}

#[test]
fn after_the_prologue_clears_the_resume_flags() {
    let steps = loadout_steps(StoryJumpPoint::AfterPrologue);
    assert!(steps.contains(&JumpStep::ClearFlag { id: 1403 }));
    assert!(steps.contains(&JumpStep::ClearFlag { id: 149 }));
}

#[test]
fn after_the_prologue_lands_on_a_known_phase_and_a_valid_point() {
    let (name, point) = destination(StoryJumpPoint::AfterPrologue);
    assert!(PhaseEntry::find(name).is_some());
    assert!((0..=rc_game::offsets::MAX_PHASE_POINT).contains(&point));
}

#[test]
fn the_popola_talk_adds_its_flags_and_keeps_the_loadout() {
    let steps = loadout_steps(StoryJumpPoint::AtPopolaTalk);
    for expected in [
        JumpStep::SetFlag { id: 71 },
        JumpStep::SetFlag { id: 153 },
        JumpStep::SetFlag { id: 154 },
        JumpStep::SetFlag { id: 1299 },
        JumpStep::SetFlag { id: 1401 },
        JumpStep::ClearFlag { id: 1403 },
        JumpStep::ClearFlag { id: 149 },
        JumpStep::EquipWeapon { id: 1000 },
        JumpStep::MpFullMode { on: false },
    ] {
        assert!(steps.contains(&expected), "{expected:?}");
    }
    assert!(!steps.contains(&JumpStep::SetFlag { id: 156 }));
}

#[test]
fn the_popola_talk_lands_in_the_library_on_a_valid_point() {
    let (name, point) = destination(StoryJumpPoint::AtPopolaTalk);
    assert_eq!(name, "A_CENTER_LIBRARY_01");
    assert!(PhaseEntry::find(name).is_some());
    assert!((0..=rc_game::offsets::MAX_PHASE_POINT).contains(&point));
}

#[test]
fn before_the_next_day_carries_the_popola_day_flags_and_the_map() {
    let steps = loadout_steps(StoryJumpPoint::BeforeNextDay);
    for expected in [
        JumpStep::SetFlag { id: 71 },
        JumpStep::SetFlag { id: 154 },
        JumpStep::SetFlag { id: 156 },
        JumpStep::SetFlag { id: 163 },
        JumpStep::SetFlag { id: 164 },
        JumpStep::SetFlag { id: 166 },
        JumpStep::SetFlag { id: 1300 },
        JumpStep::ClearFlag { id: 1299 },
        JumpStep::ClearFlag { id: 1401 },
        JumpStep::ClearFlag { id: 1403 },
        JumpStep::SetItem { id: 723, count: 1 },
        JumpStep::EquipWeapon { id: 1000 },
    ] {
        assert!(steps.contains(&expected), "{expected:?}");
    }
    assert!(!steps.contains(&JumpStep::SetFlag { id: 1299 }));
    assert!(!steps.contains(&JumpStep::SetFlag { id: 1401 }));
}

#[test]
fn before_the_next_day_lands_in_the_village_outside_the_library() {
    let (name, point) = destination(StoryJumpPoint::BeforeNextDay);
    assert_eq!(name, "A_CENTER_VILLAGE_01");
    assert_eq!(point, 4);
    assert!(PhaseEntry::find(name).is_some());
    assert!((0..=rc_game::offsets::MAX_PHASE_POINT).contains(&point));
}

#[test]
fn the_next_day_jump_gives_the_map_item_and_its_tutorial() {
    let steps = loadout_steps(StoryJumpPoint::BeforeNextDay);
    assert!(steps.contains(&JumpStep::SetItem { id: 723, count: 1 }));
    assert!(steps.contains(&JumpStep::SetTutorial { id: 2010 }));
}

#[test]
fn the_jumps_before_the_map_do_not_give_it() {
    for point in [StoryJumpPoint::AfterPrologue, StoryJumpPoint::AtPopolaTalk] {
        let steps = loadout_steps(point);
        assert!(!steps.contains(&JumpStep::SetItem { id: 723, count: 1 }));
        assert!(!steps.contains(&JumpStep::SetTutorial { id: 2010 }));
    }
}

#[test]
fn the_shrine_entrance_jump_lands_in_the_shrine_with_the_map_and_counters() {
    let steps = loadout_steps(StoryJumpPoint::AtShrineEntrance);
    for expected in [
        JumpStep::SetItem { id: 723, count: 1 },
        JumpStep::SetTutorial { id: 2010 },
        JumpStep::SetFlag { id: 167 },
        JumpStep::SetFlag { id: 171 },
        JumpStep::SetFlag { id: 77 },
        JumpStep::SetFlag { id: 1303 },
        JumpStep::SetSnowFlag { id: 1052 },
        JumpStep::SetSnowFlag { id: 1069 },
        JumpStep::ClearFlag { id: 1300 },
        JumpStep::ClearFlag { id: 1443 },
        JumpStep::SetGameCounter {
            id: 100,
            value: 255,
        },
        JumpStep::SetGameCounter {
            id: 101,
            value: 255,
        },
    ] {
        assert!(steps.contains(&expected), "{expected:?}");
    }
    for set in [190, 191] {
        assert!(steps.contains(&JumpStep::SetFlag { id: set }), "{set}");
    }
    for id in 1401..=1410 {
        assert!(steps.contains(&JumpStep::ClearFlag { id }), "{id}");
    }
    for unset in [75, 192, 202, 205] {
        assert!(!steps.contains(&JumpStep::SetFlag { id: unset }), "{unset}");
    }
    let (name, point) = destination(StoryJumpPoint::AtShrineEntrance);
    assert_eq!(name, "A_STONE_TEMPLE_01");
    assert_eq!(point, 1);
    assert!(PhaseEntry::find(name).is_some());
}

#[test]
fn every_jump_point_expects_level_zero() {
    for point in [
        StoryJumpPoint::AfterPrologue,
        StoryJumpPoint::AtPopolaTalk,
        StoryJumpPoint::BeforeNextDay,
        StoryJumpPoint::AtShrineEntrance,
    ] {
        assert_eq!(loadout_player_level(point), Some(0));
    }
}

#[test]
fn a_level_that_did_not_take_is_applied_again() {
    assert_eq!(check_level(0, Some(0)), LevelCheck::Matches);
    assert_eq!(
        check_level(0, Some(21)),
        LevelCheck::ApplyAgain {
            expected: 0,
            actual: 21
        }
    );
    assert_eq!(check_level(0, None), LevelCheck::Unreadable);
}

#[test]
fn the_watcher_reports_only_changes() {
    let mut watcher = LevelWatcher::new();
    assert_eq!(watcher.observe(21), Some((None, 21)));
    assert_eq!(watcher.observe(21), None);
    assert_eq!(watcher.observe(0), Some((Some(21), 0)));
}

#[test]
fn the_level_check_waits_for_the_new_phase_to_settle() {
    let settled = LEVEL_CHECK_SETTLE_MS;
    assert!(!level_check_is_due("A", "A", true, settled));
    assert!(!level_check_is_due("A", "B", false, settled));
    assert!(!level_check_is_due("A", "B", true, settled - 1));
    assert!(level_check_is_due("A", "B", true, settled));
}
