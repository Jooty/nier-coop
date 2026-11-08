use rc_game::PhaseEntry;
use rc_game::story_jump::{
    JumpStep, StoryJumpRefusal, StoryJumpView, check_story_jump, destination,
    jump_waits_for_pause_menu, loadout_steps,
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
