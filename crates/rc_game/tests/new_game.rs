use rc_game::new_game::{
    SAVE_WRITE_BUSY_GRACE_MS, SAVE_WRITE_TIMEOUT_MS, SaveWriteProgress, save_write_progress,
    FIRST_SAVE_GIVE_UP_MS, FIRST_SAVE_PHASE, FIRST_SAVE_SETTLE_MS, FirstSaveGate, FirstSaveInputs,
    FirstSaveStep, MENU_RESULT_NAME_CONFIRMED, MENU_RESULT_NEW_GAME, TITLE_HOLD_TIMEOUT_MS,
    TitleHold, TitleHoldInputs, TitleStep, first_save_permitted, title_hold, title_step,
};
use rc_game::player_name::{NAME_FIELD_LEN, name_field_bytes};
use rc_proto::GameReadiness;

fn ready_inputs() -> FirstSaveInputs<'static> {
    FirstSaveInputs {
        in_play: true,
        phase: FIRST_SAVE_PHASE,
        save_manager_busy: false,
    }
}

fn hold_inputs(partner_readiness: GameReadiness) -> TitleHoldInputs {
    TitleHoldInputs {
        waited_ms: 1_000,
        launcher_linked: true,
        partner_readiness,
    }
}

#[test]
fn the_result_codes_are_the_games_new_game_and_name_done() {
    assert_eq!(MENU_RESULT_NEW_GAME, 4);
    assert_eq!(MENU_RESULT_NAME_CONFIRMED, 6);
}

#[test]
fn the_title_is_skipped_only_when_the_system_data_is_loaded() {
    assert_eq!(title_step(true), TitleStep::AnswerNewGame);
    assert_eq!(title_step(false), TitleStep::ShowRealTitle);
}

#[test]
fn the_typed_name_fills_the_field_with_at_most_31_bytes_and_a_nul() {
    let field = name_field_bytes("Nier").expect("a plain name fits");
    assert_eq!(&field[..4], b"Nier");
    assert!(field[4..].iter().all(|byte| *byte == 0));
    let long = name_field_bytes(&"a".repeat(80)).expect("a long name is cut");
    assert_eq!(
        long.iter().filter(|byte| **byte != 0).count(),
        NAME_FIELD_LEN - 1
    );
    assert_eq!(long[NAME_FIELD_LEN - 1], 0);
}

#[test]
fn an_empty_name_gives_no_field_so_the_box_stays() {
    assert_eq!(name_field_bytes("   "), None);
}

#[test]
fn the_title_holds_while_the_partners_game_is_not_open() {
    assert_eq!(
        title_hold(hold_inputs(GameReadiness::NotLinked)),
        TitleHold::Hold
    );
}

#[test]
fn the_title_releases_when_the_partner_is_linked_or_in_play() {
    assert_eq!(
        title_hold(hold_inputs(GameReadiness::Loading)),
        TitleHold::Release
    );
    assert_eq!(
        title_hold(hold_inputs(GameReadiness::InPlay)),
        TitleHold::Release
    );
}

#[test]
fn the_title_holds_for_a_partner_who_is_not_connected_yet() {
    let inputs = TitleHoldInputs {
        launcher_linked: true,
        ..hold_inputs(GameReadiness::NotLinked)
    };
    assert_eq!(title_hold(inputs), TitleHold::Hold);
}

#[test]
fn the_title_holds_until_the_launcher_link_is_up() {
    let inputs = TitleHoldInputs {
        launcher_linked: false,
        ..hold_inputs(GameReadiness::NotLinked)
    };
    assert_eq!(title_hold(inputs), TitleHold::Hold);
}

#[test]
fn the_title_hold_times_out() {
    let inputs = TitleHoldInputs {
        waited_ms: TITLE_HOLD_TIMEOUT_MS,
        ..hold_inputs(GameReadiness::NotLinked)
    };
    assert_eq!(title_hold(inputs), TitleHold::TimedOut);
}

#[test]
fn an_unarmed_first_save_never_fires() {
    let mut gate = FirstSaveGate::new();
    assert_eq!(gate.step(ready_inputs(), 100_000), FirstSaveStep::Wait);
}

#[test]
fn the_first_save_waits_for_the_settle_time_then_fires_once() {
    let mut gate = FirstSaveGate::new();
    gate.arm(0);
    assert_eq!(gate.step(ready_inputs(), 1_000), FirstSaveStep::Wait);
    assert_eq!(
        gate.step(ready_inputs(), 1_000 + FIRST_SAVE_SETTLE_MS - 1),
        FirstSaveStep::Wait
    );
    assert_eq!(
        gate.step(ready_inputs(), 1_000 + FIRST_SAVE_SETTLE_MS),
        FirstSaveStep::Save
    );
    assert_eq!(gate.step(ready_inputs(), 20_000), FirstSaveStep::Done);
}

#[test]
fn a_busy_manager_or_another_phase_restarts_the_settle_time() {
    let mut gate = FirstSaveGate::new();
    gate.arm(0);
    assert_eq!(gate.step(ready_inputs(), 1_000), FirstSaveStep::Wait);
    let busy = FirstSaveInputs {
        save_manager_busy: true,
        ..ready_inputs()
    };
    assert_eq!(gate.step(busy, 2_000), FirstSaveStep::Wait);
    assert_eq!(gate.step(ready_inputs(), 3_000), FirstSaveStep::Wait);
    let elsewhere = FirstSaveInputs {
        phase: "A_STONE_FIELD_02",
        ..ready_inputs()
    };
    assert_eq!(gate.step(elsewhere, 4_000), FirstSaveStep::Wait);
    assert_eq!(gate.step(ready_inputs(), 5_000), FirstSaveStep::Wait);
    assert_eq!(
        gate.step(ready_inputs(), 5_000 + FIRST_SAVE_SETTLE_MS),
        FirstSaveStep::Save
    );
}

#[test]
fn the_first_save_gives_up_when_the_prologue_never_comes() {
    let mut gate = FirstSaveGate::new();
    gate.arm(0);
    let loading = FirstSaveInputs {
        in_play: false,
        ..ready_inputs()
    };
    assert_eq!(
        gate.step(loading, FIRST_SAVE_GIVE_UP_MS - 1),
        FirstSaveStep::Wait
    );
    assert_eq!(
        gate.step(loading, FIRST_SAVE_GIVE_UP_MS),
        FirstSaveStep::GiveUp
    );
    assert_eq!(
        gate.step(ready_inputs(), FIRST_SAVE_GIVE_UP_MS + 5_000),
        FirstSaveStep::Done
    );
}

#[test]
fn the_first_save_needs_a_link_and_the_save_block_off() {
    assert!(first_save_permitted(true, false));
    assert!(!first_save_permitted(true, true));
    assert!(!first_save_permitted(false, false));
    assert!(!first_save_permitted(false, true));
}

#[test]
fn a_save_write_is_not_done_before_the_manager_was_seen_saving() {
    assert_eq!(save_write_progress(false, false, 10), SaveWriteProgress::Wait);
    assert_eq!(
        save_write_progress(false, false, SAVE_WRITE_BUSY_GRACE_MS),
        SaveWriteProgress::Done
    );
    assert_eq!(save_write_progress(true, false, 10), SaveWriteProgress::Done);
}

#[test]
fn a_save_write_that_never_ends_times_out() {
    assert_eq!(save_write_progress(true, true, 10), SaveWriteProgress::Wait);
    assert_eq!(
        save_write_progress(true, true, SAVE_WRITE_TIMEOUT_MS),
        SaveWriteProgress::TimedOut
    );
}
