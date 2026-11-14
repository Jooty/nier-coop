use automata_game::character::Character;
use automata_game::load_gate::StoryPlace;
use automata_game::partner::{
    BODY_BACK_IN_THE_SLOT_THIS_LONG_ENDS_A_FALL_MS, BODY_PUT_BACK_RETRY_MS,
    BODY_PUT_BACK_TRIES_PER_FALL, BodyMade, BuddyReading, ENGINE_PARTNER_WAIT_MS, HeroReading,
    LOCAL_NOT_FREE_HOLD_BEFORE_RELEASE_MS, OldBodyFate, PartnerAction, PartnerMachine,
    PartnerReadings, PutBackTurn, SlotChange, SlotFall, SlotPutBack, SlotWatch,
    body_put_back_gave_up_line, body_put_back_line, given_back_body_not_put_back_line,
    given_back_body_put_back_line, old_body_line, slot_fall_line, slot_refilled_line,
    slot_took_the_body_line,
};

const HERO: u32 = 0x1100;
const BODY: u32 = 0x2200;
const OTHER_BODY: u32 = 0x9900;
const NINE_S_OBJECT_ID: u32 = 0x10200;

fn hero(handle: u32) -> Option<HeroReading> {
    Some(HeroReading {
        handle,
        player_controlled: true,
        computer_controlled: false,
    })
}

fn nine_s_in_the_slot(handle: u32) -> Option<BuddyReading> {
    Some(BuddyReading {
        handle,
        object_id: Character::NineS.object_id(),
    })
}

fn seeking(buddy: Option<BuddyReading>) -> PartnerReadings {
    PartnerReadings {
        partner_in_play: true,
        both_free: true,
        local_free_now: true,
        other_free_now: true,
        wanted: Character::NineS,
        hero: hero(HERO),
        buddy,
        other_slot_holder: None,
        held_object_id: None,
        wanted_data_is_loaded: true,
        body_keeps_its_slot: true,
        given_back_object_id: None,
    }
}

fn holding() -> PartnerReadings {
    PartnerReadings {
        held_object_id: Some(Character::NineS.object_id()),
        ..seeking(nine_s_in_the_slot(BODY))
    }
}

fn out_of_the_slot() -> PartnerReadings {
    PartnerReadings {
        buddy: None,
        ..holding()
    }
}

fn adopted() -> PartnerMachine {
    let mut machine = PartnerMachine::default();
    assert_eq!(
        machine.step(0, &seeking(nine_s_in_the_slot(BODY))).action,
        PartnerAction::Adopt
    );
    machine
}

fn spawned() -> PartnerMachine {
    let mut machine = PartnerMachine::default();
    machine.spawned(BODY, HERO);
    machine
}

fn give_up_of_an_adopted_body() -> PartnerAction {
    PartnerAction::Release {
        restore_fields: true,
        terminate: false,
        made_by_mod: false,
        mark_dismissed: true,
    }
}

fn give_up_of_a_spawned_body() -> PartnerAction {
    PartnerAction::Release {
        restore_fields: false,
        terminate: true,
        made_by_mod: true,
        mark_dismissed: false,
    }
}

fn given_back_in_a_cutscene(mut machine: PartnerMachine) -> PartnerMachine {
    let in_a_cutscene = PartnerReadings {
        both_free: false,
        local_free_now: false,
        ..holding()
    };
    assert_eq!(
        machine.step(100, &in_a_cutscene).action,
        PartnerAction::Hold
    );
    let step = machine.step(101 + LOCAL_NOT_FREE_HOLD_BEFORE_RELEASE_MS, &in_a_cutscene);
    assert!(matches!(
        step.action,
        PartnerAction::Release {
            terminate: false,
            ..
        }
    ));
    machine
}

fn free_with_the_slot_empty(given_back_object_id: Option<u32>) -> PartnerReadings {
    PartnerReadings {
        given_back_object_id,
        ..seeking(None)
    }
}

fn place(phase_id: i32, name: &str) -> StoryPlace {
    StoryPlace::new(phase_id, name)
}

#[test]
fn a_held_body_that_falls_out_of_an_empty_slot_is_put_back_at_once_with_a_line() {
    let mut machine = adopted();
    let step = machine.step(100, &out_of_the_slot());
    assert_eq!(step.action, PartnerAction::Relink);
    assert_eq!(step.logs, vec![body_put_back_line(BODY, 1)]);
    assert_eq!(machine.held_body_handle(), Some(BODY));
    assert_eq!(machine.step(116, &holding()).action, PartnerAction::Drive);
}

#[test]
fn the_put_backs_are_bounded_to_three_tries_two_hundred_ms_apart_then_the_body_is_given_up() {
    let mut machine = adopted();
    assert_eq!(
        machine.step(100, &out_of_the_slot()).action,
        PartnerAction::Relink
    );
    assert_eq!(
        machine.step(116, &out_of_the_slot()).action,
        PartnerAction::Hold
    );
    assert_eq!(
        machine
            .step(99 + BODY_PUT_BACK_RETRY_MS, &out_of_the_slot())
            .action,
        PartnerAction::Hold
    );
    let second = machine.step(100 + BODY_PUT_BACK_RETRY_MS, &out_of_the_slot());
    assert_eq!(second.action, PartnerAction::Relink);
    assert_eq!(second.logs, vec![body_put_back_line(BODY, 2)]);
    let third = machine.step(100 + 2 * BODY_PUT_BACK_RETRY_MS, &out_of_the_slot());
    assert_eq!(third.action, PartnerAction::Relink);
    assert_eq!(third.logs, vec![body_put_back_line(BODY, 3)]);
    let given_up = machine.step(100 + 3 * BODY_PUT_BACK_RETRY_MS, &out_of_the_slot());
    assert_eq!(given_up.action, give_up_of_an_adopted_body());
    assert_eq!(given_up.logs, vec![body_put_back_gave_up_line(BODY)]);
    assert_eq!(machine.held_body_handle(), None);
}

#[test]
fn a_spawned_body_whose_put_backs_ran_out_is_removed_as_today() {
    let mut machine = spawned();
    for try_index in 0..u64::from(BODY_PUT_BACK_TRIES_PER_FALL) {
        assert_eq!(
            machine
                .step(100 + try_index * BODY_PUT_BACK_RETRY_MS, &out_of_the_slot())
                .action,
            PartnerAction::Relink
        );
    }
    let last_ms = 100 + u64::from(BODY_PUT_BACK_TRIES_PER_FALL) * BODY_PUT_BACK_RETRY_MS;
    assert_eq!(
        machine.step(last_ms, &out_of_the_slot()).action,
        give_up_of_a_spawned_body()
    );
}

#[test]
fn a_body_back_in_the_slot_for_long_enough_gets_three_new_tries_at_its_next_fall() {
    let mut machine = adopted();
    let mut now_ms = 100;
    for _ in 0..BODY_PUT_BACK_TRIES_PER_FALL {
        assert_eq!(
            machine.step(now_ms, &out_of_the_slot()).action,
            PartnerAction::Relink
        );
        assert_eq!(
            machine.step(now_ms + 16, &holding()).action,
            PartnerAction::Drive
        );
        assert_eq!(
            machine
                .step(
                    now_ms + 16 + BODY_BACK_IN_THE_SLOT_THIS_LONG_ENDS_A_FALL_MS,
                    &holding()
                )
                .action,
            PartnerAction::Drive
        );
        now_ms += 1_000;
    }
    let fourth_fall = machine.step(now_ms, &out_of_the_slot());
    assert_eq!(fourth_fall.action, PartnerAction::Relink);
    assert_eq!(fourth_fall.logs, vec![body_put_back_line(BODY, 1)]);
}

#[test]
fn a_body_that_keeps_falling_out_at_once_runs_out_of_tries() {
    let mut machine = adopted();
    for try_index in 0..u64::from(BODY_PUT_BACK_TRIES_PER_FALL) {
        let try_ms = 100 + try_index * BODY_PUT_BACK_RETRY_MS;
        assert_eq!(
            machine.step(try_ms, &out_of_the_slot()).action,
            PartnerAction::Relink
        );
        assert_eq!(
            machine.step(try_ms + 16, &holding()).action,
            PartnerAction::Drive
        );
    }
    let last_ms = 100 + u64::from(BODY_PUT_BACK_TRIES_PER_FALL) * BODY_PUT_BACK_RETRY_MS;
    assert_eq!(
        machine.step(last_ms, &out_of_the_slot()).action,
        give_up_of_an_adopted_body()
    );
}

#[test]
fn a_slot_the_game_refilled_with_another_body_is_not_fought_and_the_holder_is_named() {
    let refilled = PartnerReadings {
        buddy: nine_s_in_the_slot(OTHER_BODY),
        ..holding()
    };
    let mut machine = adopted();
    let step = machine.step(100, &refilled);
    assert_eq!(step.action, give_up_of_an_adopted_body());
    assert_eq!(
        step.logs,
        vec![slot_refilled_line(BODY, OTHER_BODY, Some(NINE_S_OBJECT_ID))]
    );
    let next = machine.step(116, &seeking(nine_s_in_the_slot(OTHER_BODY)));
    assert_eq!(next.action, PartnerAction::Adopt);
    assert!(next.logs.contains(&old_body_line(
        BodyMade::TakenOver,
        OldBodyFate::SlotHeldAnother { handle: BODY }
    )));
    assert_eq!(machine.held_body_handle(), Some(OTHER_BODY));
}

#[test]
fn a_slot_refilled_with_something_that_is_not_a_player_body_is_not_linked_over() {
    let refilled = PartnerReadings {
        buddy: None,
        other_slot_holder: Some(OTHER_BODY),
        ..holding()
    };
    let mut machine = spawned();
    let step = machine.step(100, &refilled);
    assert_eq!(step.action, give_up_of_a_spawned_body());
    assert_eq!(step.logs, vec![slot_refilled_line(BODY, OTHER_BODY, None)]);
}

#[test]
fn with_the_setting_off_a_fall_is_linked_again_once_and_then_given_up_as_today() {
    let off = PartnerReadings {
        body_keeps_its_slot: false,
        ..out_of_the_slot()
    };
    let mut machine = adopted();
    let first = machine.step(100, &off);
    assert_eq!(first.action, PartnerAction::Relink);
    assert!(first.logs[0].contains("linking it again"));
    assert_eq!(machine.step(116, &off).action, give_up_of_an_adopted_body());
}

#[test]
fn with_the_setting_off_a_body_given_back_is_not_remembered_and_no_reason_line_is_added() {
    let off = |readings: PartnerReadings| PartnerReadings {
        body_keeps_its_slot: false,
        ..readings
    };
    let mut machine = adopted();
    let in_a_cutscene = off(PartnerReadings {
        both_free: false,
        local_free_now: false,
        ..holding()
    });
    machine.step(100, &in_a_cutscene);
    machine.step(101 + LOCAL_NOT_FREE_HOLD_BEFORE_RELEASE_MS, &in_a_cutscene);
    assert_eq!(machine.given_back_body_handle(), None);
    let empty = off(free_with_the_slot_empty(Some(NINE_S_OBJECT_ID)));
    assert_eq!(machine.step(2_000, &empty).action, PartnerAction::Nothing);
    let spawn = machine.step(2_000 + ENGINE_PARTNER_WAIT_MS, &empty);
    assert_eq!(
        spawn.action,
        PartnerAction::Spawn {
            object_id: NINE_S_OBJECT_ID
        }
    );
    assert_eq!(spawn.logs.len(), 1);
}

#[test]
fn a_body_given_back_in_a_cutscene_is_remembered() {
    let machine = given_back_in_a_cutscene(adopted());
    assert_eq!(machine.held_body_handle(), None);
    assert_eq!(machine.given_back_body_handle(), Some(BODY));
}

#[test]
fn a_given_back_body_is_not_put_back_while_a_player_is_not_free() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let still_held = PartnerReadings {
        both_free: false,
        ..free_with_the_slot_empty(Some(NINE_S_OBJECT_ID))
    };
    assert_eq!(
        machine.step(1_500, &still_held).action,
        PartnerAction::Nothing
    );
    assert_eq!(machine.given_back_body_handle(), Some(BODY));
}

#[test]
fn a_given_back_body_that_left_the_slot_and_still_lives_is_put_back_before_any_wait_or_spawn() {
    let mut machine = given_back_in_a_cutscene(spawned());
    let step = machine.step(2_000, &free_with_the_slot_empty(Some(NINE_S_OBJECT_ID)));
    assert_eq!(step.action, PartnerAction::PutBack { body_handle: BODY });
    assert_eq!(step.logs, vec![given_back_body_put_back_line(BODY, 1)]);
    let back = machine.step(2_016, &seeking(nine_s_in_the_slot(BODY)));
    assert_eq!(back.action, PartnerAction::Adopt);
    assert!(back.logs.contains(&old_body_line(
        BodyMade::TakenOver,
        OldBodyFate::Kept { handle: BODY }
    )));
    assert_eq!(machine.held_body_handle(), Some(BODY));
    assert_eq!(machine.given_back_body_handle(), None);
}

#[test]
fn a_put_back_spawned_body_is_still_known_as_made_by_the_mod() {
    let mut machine = given_back_in_a_cutscene(spawned());
    machine.step(2_000, &free_with_the_slot_empty(Some(NINE_S_OBJECT_ID)));
    machine.step(2_016, &seeking(nine_s_in_the_slot(BODY)));
    let refilled = PartnerReadings {
        buddy: nine_s_in_the_slot(OTHER_BODY),
        ..holding()
    };
    assert_eq!(
        machine.step(2_032, &refilled).action,
        give_up_of_a_spawned_body()
    );
}

#[test]
fn a_given_back_body_still_in_the_slot_is_taken_over_and_named_as_kept() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let step = machine.step(2_000, &seeking(nine_s_in_the_slot(BODY)));
    assert_eq!(step.action, PartnerAction::Adopt);
    assert!(step.logs.contains(&old_body_line(
        BodyMade::TakenOver,
        OldBodyFate::Kept { handle: BODY }
    )));
}

#[test]
fn a_given_back_body_that_is_gone_leads_to_the_wait_and_the_spawn_of_today_with_the_reason() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let empty = free_with_the_slot_empty(None);
    let step = machine.step(2_000, &empty);
    assert_eq!(step.action, PartnerAction::Nothing);
    assert_eq!(
        step.logs[0],
        given_back_body_not_put_back_line(OldBodyFate::Gone { handle: BODY })
    );
    assert!(step.logs[1].contains("waiting up to 5 s"));
    assert_eq!(machine.given_back_body_handle(), None);
    let spawn = machine.step(2_000 + ENGINE_PARTNER_WAIT_MS, &empty);
    assert_eq!(
        spawn.action,
        PartnerAction::Spawn {
            object_id: NINE_S_OBJECT_ID
        }
    );
    assert!(spawn.logs.contains(&old_body_line(
        BodyMade::Spawned,
        OldBodyFate::Gone { handle: BODY }
    )));
}

#[test]
fn a_given_back_body_of_another_character_is_not_put_back() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let step = machine.step(2_000, &free_with_the_slot_empty(Some(0x10000)));
    assert_eq!(step.action, PartnerAction::Nothing);
    assert_eq!(
        step.logs[0],
        given_back_body_not_put_back_line(OldBodyFate::OtherCharacter {
            handle: BODY,
            object_id: 0x10000
        })
    );
}

#[test]
fn a_given_back_body_of_another_hero_is_not_put_back() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let new_hero = PartnerReadings {
        hero: hero(0x3300),
        ..free_with_the_slot_empty(Some(NINE_S_OBJECT_ID))
    };
    let step = machine.step(2_000, &new_hero);
    assert_eq!(step.action, PartnerAction::Nothing);
    assert_eq!(
        step.logs[0],
        given_back_body_not_put_back_line(OldBodyFate::OtherHero { handle: BODY })
    );
}

#[test]
fn a_given_back_body_that_became_the_hero_is_not_put_back() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let switched = PartnerReadings {
        hero: hero(BODY),
        ..free_with_the_slot_empty(Some(NINE_S_OBJECT_ID))
    };
    let step = machine.step(2_000, &switched);
    assert_eq!(
        step.logs[0],
        given_back_body_not_put_back_line(OldBodyFate::IsTheHeroNow { handle: BODY })
    );
}

#[test]
fn a_given_back_body_the_slot_will_not_take_is_tried_three_times_then_todays_path_follows() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let empty = free_with_the_slot_empty(Some(NINE_S_OBJECT_ID));
    for try_index in 0..u64::from(BODY_PUT_BACK_TRIES_PER_FALL) {
        let try_ms = 2_000 + try_index * BODY_PUT_BACK_RETRY_MS;
        assert_eq!(
            machine.step(try_ms, &empty).action,
            PartnerAction::PutBack { body_handle: BODY }
        );
        assert_eq!(
            machine.step(try_ms + 16, &empty).action,
            PartnerAction::Nothing
        );
    }
    let out_ms = 2_000 + u64::from(BODY_PUT_BACK_TRIES_PER_FALL) * BODY_PUT_BACK_RETRY_MS;
    let step = machine.step(out_ms, &empty);
    assert_eq!(
        step.logs[0],
        given_back_body_not_put_back_line(OldBodyFate::OutOfTries { handle: BODY })
    );
    assert_eq!(
        machine.step(out_ms + ENGINE_PARTNER_WAIT_MS, &empty).action,
        PartnerAction::Spawn {
            object_id: NINE_S_OBJECT_ID
        }
    );
}

#[test]
fn an_engine_body_in_the_slot_in_place_of_the_given_back_one_is_taken_over_as_today() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let step = machine.step(2_000, &seeking(nine_s_in_the_slot(OTHER_BODY)));
    assert_eq!(step.action, PartnerAction::Adopt);
    assert!(step.logs.contains(&old_body_line(
        BodyMade::TakenOver,
        OldBodyFate::SlotHeldAnother { handle: BODY }
    )));
    assert_eq!(machine.held_body_handle(), Some(OTHER_BODY));
}

#[test]
fn a_given_back_body_is_forgotten_when_the_partner_stops_being_in_play() {
    let mut machine = given_back_in_a_cutscene(adopted());
    let not_in_play = PartnerReadings {
        partner_in_play: false,
        ..free_with_the_slot_empty(Some(NINE_S_OBJECT_ID))
    };
    assert_eq!(
        machine.step(2_000, &not_in_play).action,
        PartnerAction::Nothing
    );
    assert_eq!(machine.given_back_body_handle(), None);
}

#[test]
fn a_new_body_made_with_no_old_one_known_says_so() {
    let mut machine = PartnerMachine::default();
    let empty = seeking(None);
    machine.step(0, &empty);
    let spawn = machine.step(ENGINE_PARTNER_WAIT_MS, &empty);
    assert!(
        spawn
            .logs
            .contains(&old_body_line(BodyMade::Spawned, OldBodyFate::NoneKnown))
    );
}

#[test]
fn a_put_back_waits_two_hundred_ms_between_tries_and_counts_them() {
    let mut put_back = SlotPutBack::default();
    assert_eq!(put_back.turn(0), PutBackTurn::Try { number: 1 });
    assert_eq!(put_back.turn(BODY_PUT_BACK_RETRY_MS - 1), PutBackTurn::Wait);
    assert_eq!(
        put_back.turn(BODY_PUT_BACK_RETRY_MS),
        PutBackTurn::Try { number: 2 }
    );
    assert_eq!(
        put_back.turn(2 * BODY_PUT_BACK_RETRY_MS),
        PutBackTurn::Try { number: 3 }
    );
    assert_eq!(put_back.tries(), 3);
    assert_eq!(
        put_back.turn(3 * BODY_PUT_BACK_RETRY_MS),
        PutBackTurn::OutOfTries
    );
}

#[test]
fn a_slot_watch_names_a_fall_with_the_slot_before_and_after_and_the_story_places() {
    let mut watch = SlotWatch::default();
    let first = place(0x100, "00_40_A");
    let second = place(0x100, "00_50_A");
    assert_eq!(watch.observe(0, Some(BODY), Some(BODY), first), None);
    assert_eq!(watch.observe(1_000, Some(BODY), Some(BODY), second), None);
    assert_eq!(
        watch.observe(1_037, Some(0), Some(BODY), second),
        Some(SlotChange {
            body_handle: BODY,
            slot_before: BODY,
            slot_after: 0,
            place_before: first,
            place_now: second,
            since_place_change_ms: Some(37),
        })
    );
    assert_eq!(watch.observe(1_053, Some(0), Some(BODY), second), None);
}

#[test]
fn a_slot_watch_sees_no_fall_without_a_hero_or_a_watched_body() {
    let mut watch = SlotWatch::default();
    let here = place(0x100, "00_40_A");
    watch.observe(0, Some(BODY), Some(BODY), here);
    assert_eq!(watch.observe(16, None, Some(BODY), here), None);
    watch.observe(32, Some(BODY), None, here);
    assert_eq!(watch.observe(48, Some(0), None, here), None);
}

#[test]
fn the_slot_fall_line_says_every_reading() {
    let fall = SlotFall {
        change: SlotChange {
            body_handle: 0x121_8100,
            slot_before: 0x121_8100,
            slot_after: 0,
            place_before: place(0x100, "00_70_A"),
            place_now: place(0x100, "00_80_A"),
            since_place_change_ms: Some(37),
        },
        given_back: false,
        body_resolves: true,
        body_state: Some(0x46),
    };
    assert_eq!(
        slot_fall_line(&fall),
        "partner: slot fall: the hero's partner slot went from 0x1218100 to 0x0; the held body 0x1218100 resolves true, state 0x46; story place 0x100 00_70_A -> 0x100 00_80_A, 37 ms since the story place last changed"
    );
    let unknown = SlotFall {
        change: SlotChange {
            since_place_change_ms: None,
            ..fall.change
        },
        given_back: true,
        body_resolves: false,
        body_state: None,
    };
    assert!(slot_fall_line(&unknown).contains(
        "the given-back body 0x1218100 resolves false, state unreadable; story place 0x100 00_70_A -> 0x100 00_80_A, no story-place change seen yet"
    ));
}

#[test]
fn the_put_back_lines_say_the_handle_and_the_bound() {
    assert_eq!(
        body_put_back_line(BODY, 2),
        "partner: the body (handle 0x2200) fell out of the hero's partner slot and is still alive, put it back (try 2 of 3, 200 ms apart)"
    );
    assert_eq!(
        body_put_back_gave_up_line(BODY),
        "partner: the body (handle 0x2200) did not stay in the hero's partner slot after 3 put-backs 200 ms apart, giving it up and starting over"
    );
    assert_eq!(
        given_back_body_put_back_line(BODY, 1),
        "partner: put back the body given back to the game (handle 0x2200), which left the hero's partner slot while given back (try 1 of 3, 200 ms apart)"
    );
    assert_eq!(
        slot_took_the_body_line(BODY, true),
        "partner: the hero's partner slot holds the body (handle 0x2200) again"
    );
    assert_eq!(
        slot_took_the_body_line(BODY, false),
        "partner: the hero's partner slot did not take the body (handle 0x2200) this time"
    );
}

#[test]
fn the_refilled_line_names_the_body_in_the_slot() {
    assert_eq!(
        slot_refilled_line(BODY, OTHER_BODY, Some(NINE_S_OBJECT_ID)),
        "partner: the game put another body into the hero's partner slot (handle 0x9900, object 0x10200) in place of ours (handle 0x2200); not fighting it, letting ours go"
    );
    assert!(
        slot_refilled_line(BODY, OTHER_BODY, None).contains("(handle 0x9900, not a player body)")
    );
}

#[test]
fn the_old_body_lines_say_why_it_was_not_kept() {
    assert_eq!(
        old_body_line(BodyMade::Spawned, OldBodyFate::Gone { handle: BODY }),
        "partner: making a new body: the old one (handle 0x2200) no longer resolves"
    );
    assert_eq!(
        old_body_line(BodyMade::TakenOver, OldBodyFate::Kept { handle: BODY }),
        "partner: taking over the body in the slot: the old one (handle 0x2200) is the body in the slot, kept"
    );
    assert_eq!(
        given_back_body_not_put_back_line(OldBodyFate::OutOfTries { handle: BODY }),
        "partner: the body given back to the game is not put back: the old one (handle 0x2200) did not stay in the slot after 3 put-backs"
    );
    assert!(
        OldBodyFate::NoneKnown
            .describe()
            .starts_with("no old one is known")
    );
    assert!(
        OldBodyFate::OtherCharacter {
            handle: BODY,
            object_id: 0x10000
        }
        .describe()
        .contains("is object 0x10000, not the wanted character")
    );
    assert!(
        OldBodyFate::OtherHero { handle: BODY }
            .describe()
            .contains("another hero")
    );
    assert!(
        OldBodyFate::IsTheHeroNow { handle: BODY }
            .describe()
            .contains("is the hero now")
    );
    assert!(
        OldBodyFate::SlotHeldAnother { handle: BODY }
            .describe()
            .contains("is left where it is")
    );
}
