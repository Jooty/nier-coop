use automata_game::address::{CAMERA_MODE_REQUEST_FLAG_WORD_VALUE, GAME_OVER_MENU_OBJECT};
use automata_game::defeat::{DefeatFlow, RestorePoint};
use automata_game::exe::ExeVariant;
use automata_game::load_gate::StoryPlace;
use automata_game::shared_defeat::{
    DefeatNote, DefeatWay, GAME_OVER_MENU_CURSOR_OFFSET, GAME_OVER_MENU_STATE_OFFSET,
    GameOverChoice, GameOverMenu, GameOverMenuWrite, HEARD_DEFEAT_NOTE_KEPT_FOR_MS,
    IN_PLACE_RETRY_BIT, PartnerBodyGuardAct, PartnerBodyReading, RETRY_PHASE_CHANGE_PARAMS,
    RestorePlace, SharedDefeatMachine, SharedDefeatReadings, follow_line,
    heard_choice_gave_up_line, partner_body_guard_act, partner_body_guard_line,
    retry_not_followed_line,
};
use automata_game::travel::JumpRole;

const MENU_CHOOSING: GameOverMenu = GameOverMenu {
    done: 0,
    state: 3,
    sub_state: 2,
    cursor: 0,
};

fn flow(state: i32, step: i32) -> DefeatFlow {
    DefeatFlow { state, step }
}

fn place(phase_id: i32, name: &str) -> RestorePlace {
    RestorePlace {
        place: StoryPlace::new(phase_id, name),
        has_a_point: true,
    }
}

fn readings(now_ms: u64, flow: DefeatFlow, menu: GameOverMenu) -> SharedDefeatReadings {
    SharedDefeatReadings {
        now_ms,
        role: JumpRole::Host,
        campaign_partner_present: true,
        flow: Some(flow),
        menu: Some(menu),
        in_place_retry: true,
        restore_point: RestorePoint::Present,
        restore_place: Some(place(0x100, "p_a")),
        heard: Vec::new(),
        local_unattended: false,
    }
}

fn in_the_menu(now_ms: u64, menu: GameOverMenu) -> SharedDefeatReadings {
    readings(now_ms, flow(5, 4), menu)
}

fn heard(mut readings: SharedDefeatReadings, note: DefeatNote) -> SharedDefeatReadings {
    readings.heard.push(note);
    readings
}

fn retry_chosen() -> GameOverMenu {
    GameOverMenu {
        state: 6,
        ..MENU_CHOOSING
    }
}

fn title_chosen() -> GameOverMenu {
    GameOverMenu {
        state: 5,
        cursor: 1,
        ..MENU_CHOOSING
    }
}

#[test]
fn every_state_of_the_flow_has_its_own_words() {
    let texts: Vec<String> = (0..=8).map(|state| flow(state, 0).describe()).collect();
    for (index, text) in texts.iter().enumerate() {
        assert!(!text.contains("another kind of flow"), "{text}");
        assert!(texts.iter().skip(index + 1).all(|later| later != text));
    }
    assert!(flow(4, 10).describe().starts_with("state 4"));
    assert!(flow(4, 10).describe().contains("restore point"));
    assert!(flow(5, 4).describe().contains("game-over menu"));
    assert!(flow(6, 0).describe().contains("0xF30"));
}

#[test]
fn step_11_names_the_word_it_waits_for_and_step_12_ends_the_flow() {
    for state in [3, 4] {
        assert!(flow(state, 11).describe().contains("0xDFA821B1"));
        assert!(!flow(state, 11).describe().contains("fade"));
        assert!(flow(state, 12).describe().contains("flow ends"));
    }
}

#[test]
fn every_step_of_the_game_over_menu_state_has_its_own_words() {
    let texts: Vec<String> = (0..=5).map(|step| flow(5, step).describe()).collect();
    for (index, text) in texts.iter().enumerate() {
        assert!(text.starts_with("state 5"));
        assert!(texts.iter().skip(index + 1).all(|later| later != text));
    }
    assert!(flow(5, 4).waits_in_the_game_over_menu());
    assert!(!flow(5, 3).waits_in_the_game_over_menu());
    assert!(!flow(3, 4).waits_in_the_game_over_menu());
}

#[test]
fn the_way_on_is_states_3_to_7_of_the_flow() {
    assert_eq!(
        DefeatWay::of_flow(flow(3, 0)),
        Some(DefeatWay::RestorePoint)
    );
    assert_eq!(
        DefeatWay::of_flow(flow(4, 0)),
        Some(DefeatWay::RestorePointWithoutBodyScreen)
    );
    assert_eq!(
        DefeatWay::of_flow(flow(5, 0)),
        Some(DefeatWay::GameOverMenu)
    );
    assert_eq!(
        DefeatWay::of_flow(flow(6, 0)),
        Some(DefeatWay::FixedPhaseRequest)
    );
    assert_eq!(DefeatWay::of_flow(flow(7, 0)), Some(DefeatWay::Title));
    for state in [0, 1, 2, 8] {
        assert_eq!(DefeatWay::of_flow(flow(state, 0)), None);
    }
}

#[test]
fn the_menus_own_choice_is_read_from_its_state_change_out_of_choosing() {
    assert_eq!(
        retry_chosen().choice_since(MENU_CHOOSING),
        Some(GameOverChoice::Retry)
    );
    let second_list = GameOverMenu {
        state: 4,
        ..MENU_CHOOSING
    };
    assert_eq!(
        second_list.choice_since(MENU_CHOOSING),
        Some(GameOverChoice::Retry)
    );
    assert_eq!(
        title_chosen().choice_since(MENU_CHOOSING),
        Some(GameOverChoice::Title)
    );
    assert_eq!(MENU_CHOOSING.choice_since(MENU_CHOOSING), None);
    assert_eq!(title_chosen().choice_since(second_list), None);
    let title_dialog = GameOverMenu {
        sub_state: 3,
        cursor: 1,
        ..MENU_CHOOSING
    };
    assert!(!title_dialog.is_choosing());
    assert!(MENU_CHOOSING.is_choosing());
}

#[test]
fn the_menu_words_are_the_ones_of_the_ledger_on_both_exes() {
    assert_eq!(GAME_OVER_MENU_OBJECT.rva(ExeVariant::Main), 0x129_6B50);
    assert_eq!(GAME_OVER_MENU_OBJECT.rva(ExeVariant::Compat), 0x128_A9D0);
    for (variant, state, cursor) in [
        (ExeVariant::Main, 0x129_6B58, 0x129_6C60),
        (ExeVariant::Compat, 0x128_A9D8, 0x128_AAE0),
    ] {
        let menu = u64::from(GAME_OVER_MENU_OBJECT.rva(variant));
        assert_eq!(menu + GAME_OVER_MENU_STATE_OFFSET, state);
        assert_eq!(menu + GAME_OVER_MENU_CURSOR_OFFSET, cursor);
    }
    assert_eq!(
        CAMERA_MODE_REQUEST_FLAG_WORD_VALUE.rva(ExeVariant::Main),
        0x102_9B04
    );
    assert_eq!(
        CAMERA_MODE_REQUEST_FLAG_WORD_VALUE.rva(ExeVariant::Compat),
        0x101_D984
    );
    assert_eq!(IN_PLACE_RETRY_BIT, 0x1000_0000);
    assert_eq!(RETRY_PHASE_CHANGE_PARAMS, [1, 0, 0x78, 0, 0, 0]);
}

#[test]
fn the_first_own_choice_is_told_once() {
    let mut machine = SharedDefeatMachine::new();
    machine.step(&in_the_menu(0, MENU_CHOOSING));
    let step = machine.step(&in_the_menu(100, retry_chosen()));
    assert_eq!(
        step.tell,
        vec![DefeatNote::MenuChoice(GameOverChoice::Retry)]
    );
    assert_eq!(step.menu_write, None);
    let step = machine.step(&in_the_menu(200, MENU_CHOOSING));
    let step2 = machine.step(&in_the_menu(300, title_chosen()));
    assert!(step.tell.is_empty() && step2.tell.is_empty());
}

#[test]
fn a_heard_title_is_made_in_a_menu_that_waits_for_a_choice() {
    let mut machine = SharedDefeatMachine::new();
    machine.step(&in_the_menu(0, MENU_CHOOSING));
    let step = machine.step(&heard(
        in_the_menu(100, MENU_CHOOSING),
        DefeatNote::MenuChoice(GameOverChoice::Title),
    ));
    assert_eq!(step.menu_write, Some(GameOverMenuWrite::Title));
    assert!(step.logs.contains(&follow_line(
        GameOverChoice::Title,
        GameOverMenuWrite::Title,
        0,
        false
    )));
}

#[test]
fn a_heard_retry_is_made_in_place_only_while_the_in_place_retry_bit_is_set() {
    let mut machine = SharedDefeatMachine::new();
    let step = machine.step(&heard(
        in_the_menu(0, MENU_CHOOSING),
        DefeatNote::MenuChoice(GameOverChoice::Retry),
    ));
    assert_eq!(step.menu_write, Some(GameOverMenuWrite::RetryInPlace));

    let mut machine = SharedDefeatMachine::new();
    let mut without_the_bit = in_the_menu(0, MENU_CHOOSING);
    without_the_bit.in_place_retry = false;
    let step = machine.step(&heard(
        without_the_bit.clone(),
        DefeatNote::MenuChoice(GameOverChoice::Retry),
    ));
    assert_eq!(step.menu_write, None);
    assert!(step.logs.contains(&retry_not_followed_line()));
    without_the_bit.now_ms = 100;
    assert_eq!(machine.step(&without_the_bit).menu_write, None);
}

#[test]
fn the_copy_that_follows_does_not_tell_the_choice_back() {
    let mut machine = SharedDefeatMachine::new();
    machine.step(&heard(
        in_the_menu(0, MENU_CHOOSING),
        DefeatNote::MenuChoice(GameOverChoice::Title),
    ));
    let step = machine.step(&in_the_menu(100, title_chosen()));
    assert!(step.tell.is_empty());
    assert_eq!(step.menu_write, None);
}

#[test]
fn a_choice_heard_before_the_menu_opens_is_made_once_the_menu_waits() {
    let mut machine = SharedDefeatMachine::new();
    let step = machine.step(&heard(
        readings(0, flow(2, 0), GameOverMenu::default()),
        DefeatNote::MenuChoice(GameOverChoice::Title),
    ));
    assert_eq!(step.menu_write, None);
    assert_eq!(machine.heard_choice(), Some(GameOverChoice::Title));
    let opening = GameOverMenu {
        sub_state: 0,
        ..MENU_CHOOSING
    };
    assert_eq!(
        machine.step(&readings(500, flow(5, 3), opening)).menu_write,
        None
    );
    assert_eq!(machine.step(&in_the_menu(900, opening)).menu_write, None);
    let step = machine.step(&in_the_menu(1_200, MENU_CHOOSING));
    assert_eq!(step.menu_write, Some(GameOverMenuWrite::Title));
}

#[test]
fn the_stale_words_of_the_last_menu_are_never_read_as_a_choice() {
    let mut machine = SharedDefeatMachine::new();
    let last_menu = GameOverMenu {
        done: 1,
        state: 6,
        sub_state: 2,
        cursor: 0,
    };
    for (now_ms, step) in [(0, 0), (100, 1), (200, 2)] {
        let step = machine.step(&readings(now_ms, flow(5, step), last_menu));
        assert!(
            step.tell
                .iter()
                .all(|note| !matches!(note, DefeatNote::MenuChoice(_)))
        );
    }
    machine.step(&readings(300, flow(5, 2), MENU_CHOOSING));
    let step = machine.step(&readings(400, flow(5, 3), MENU_CHOOSING));
    assert!(
        step.tell
            .iter()
            .all(|note| !matches!(note, DefeatNote::MenuChoice(_)))
    );
}

#[test]
fn a_heard_choice_is_kept_for_a_bound_with_a_line_a_second_then_dropped() {
    let mut machine = SharedDefeatMachine::new();
    let waiting = |now_ms| readings(now_ms, flow(1, 0), GameOverMenu::default());
    machine.step(&heard(
        waiting(0),
        DefeatNote::MenuChoice(GameOverChoice::Retry),
    ));
    assert!(machine.step(&waiting(500)).logs.is_empty());
    let step = machine.step(&waiting(1_000));
    assert_eq!(step.logs.len(), 1);
    assert!(step.logs[0].contains("waits 1000 ms"));
    let step = machine.step(&waiting(HEARD_DEFEAT_NOTE_KEPT_FOR_MS + 1));
    assert!(step.logs.contains(&heard_choice_gave_up_line(
        GameOverChoice::Retry,
        Some(flow(1, 0))
    )));
    assert_eq!(machine.heard_choice(), None);
    let step = machine.step(&in_the_menu(
        HEARD_DEFEAT_NOTE_KEPT_FOR_MS + 100,
        MENU_CHOOSING,
    ));
    assert_eq!(step.menu_write, None);
}

#[test]
fn a_copy_in_its_menu_that_hears_nothing_keeps_its_own_menu() {
    let mut machine = SharedDefeatMachine::new();
    for now_ms in (0..120_000).step_by(1_000) {
        let step = machine.step(&in_the_menu(now_ms, MENU_CHOOSING));
        assert_eq!(step.menu_write, None);
        assert!(step.logs.is_empty() || now_ms == 0);
    }
}

#[test]
fn a_choice_heard_after_this_copy_chose_is_not_written() {
    let mut machine = SharedDefeatMachine::new();
    machine.step(&in_the_menu(0, MENU_CHOOSING));
    machine.step(&in_the_menu(100, title_chosen()));
    let step = machine.step(&heard(
        in_the_menu(200, title_chosen()),
        DefeatNote::MenuChoice(GameOverChoice::Retry),
    ));
    assert_eq!(step.menu_write, None);
    assert_eq!(machine.heard_choice(), None);
}

#[test]
fn this_copys_own_choice_first_drops_a_heard_one_not_yet_made() {
    let mut machine = SharedDefeatMachine::new();
    let mut dialog = MENU_CHOOSING;
    dialog.sub_state = 3;
    machine.step(&heard(
        in_the_menu(0, dialog),
        DefeatNote::MenuChoice(GameOverChoice::Retry),
    ));
    let step = machine.step(&in_the_menu(100, title_chosen()));
    assert_eq!(
        step.tell,
        vec![DefeatNote::MenuChoice(GameOverChoice::Title)]
    );
    assert_eq!(step.menu_write, None);
    assert_eq!(machine.heard_choice(), None);
}

#[test]
fn each_way_on_is_told_once_with_the_restore_place() {
    let mut machine = SharedDefeatMachine::new();
    assert!(
        machine
            .step(&readings(0, flow(1, 0), MENU_CHOOSING))
            .tell
            .is_empty()
    );
    let step = machine.step(&readings(100, flow(3, 0), MENU_CHOOSING));
    assert_eq!(
        step.tell,
        vec![DefeatNote::Way {
            way: DefeatWay::RestorePoint,
            restore: Some(place(0x100, "p_a")),
        }]
    );
    assert!(
        machine
            .step(&readings(200, flow(3, 10), MENU_CHOOSING))
            .tell
            .is_empty()
    );
    let mut no_restore = readings(300, flow(7, 0), MENU_CHOOSING);
    no_restore.restore_point = RestorePoint::Absent;
    no_restore.restore_place = None;
    let step = machine.step(&no_restore);
    assert_eq!(
        step.tell,
        vec![DefeatNote::Way {
            way: DefeatWay::Title,
            restore: None,
        }]
    );
    machine.step(&readings(400, flow(0, 0), MENU_CHOOSING));
    let step = machine.step(&readings(500, flow(3, 0), MENU_CHOOSING));
    assert_eq!(step.tell.len(), 1);
}

#[test]
fn a_heard_way_logs_both_restore_places_and_names_the_catch_up_when_they_differ() {
    let mut machine = SharedDefeatMachine::new();
    let mut guest = readings(0, flow(3, 0), MENU_CHOOSING);
    guest.role = JumpRole::Guest;
    machine.step(&guest);
    guest.now_ms = 100;
    let step = machine.step(&heard(
        guest,
        DefeatNote::Way {
            way: DefeatWay::RestorePoint,
            restore: Some(place(0x200, "p_b")),
        },
    ));
    let line = &step.logs[0];
    assert!(line.contains("0x200 p_b"), "{line}");
    assert!(line.contains("0x100 p_a"), "{line}");
    assert!(line.contains("catch-up brings this guest"), "{line}");
}

#[test]
fn ways_that_differ_are_logged_and_left_alone() {
    let mut machine = SharedDefeatMachine::new();
    machine.step(&readings(0, flow(3, 0), MENU_CHOOSING));
    let step = machine.step(&heard(
        readings(100, flow(3, 10), MENU_CHOOSING),
        DefeatNote::Way {
            way: DefeatWay::Title,
            restore: None,
        },
    ));
    assert!(step.logs.iter().any(|line| line.contains("other ways on")));
    assert!(step.tell.is_empty() && step.menu_write.is_none());
}

#[test]
fn a_heard_choice_is_dropped_when_this_copys_flow_takes_no_menu() {
    let mut machine = SharedDefeatMachine::new();
    machine.step(&heard(
        readings(0, flow(2, 0), MENU_CHOOSING),
        DefeatNote::MenuChoice(GameOverChoice::Title),
    ));
    machine.step(&readings(100, flow(4, 0), MENU_CHOOSING));
    assert_eq!(machine.heard_choice(), None);
}

#[test]
fn without_a_campaign_partner_nothing_is_told_heard_or_written() {
    for (role, present) in [
        (JumpRole::Solo, true),
        (JumpRole::Host, false),
        (JumpRole::Guest, false),
    ] {
        let mut machine = SharedDefeatMachine::new();
        let alone = |now_ms, menu| {
            let mut readings = in_the_menu(now_ms, menu);
            readings.role = role;
            readings.campaign_partner_present = present;
            readings
        };
        let first = machine.step(&heard(
            alone(0, MENU_CHOOSING),
            DefeatNote::MenuChoice(GameOverChoice::Title),
        ));
        let second = machine.step(&alone(100, retry_chosen()));
        for step in [first, second] {
            assert!(step.tell.is_empty() && step.menu_write.is_none() && step.logs.is_empty());
        }
        assert_eq!(machine.heard_choice(), None);
    }
}

#[test]
fn a_note_with_a_bad_place_name_is_not_valid() {
    let good = DefeatNote::Way {
        way: DefeatWay::RestorePoint,
        restore: Some(place(0x100, "p_a")),
    };
    assert!(good.is_valid());
    let mut bad_place = place(0x100, "p_a");
    bad_place.place.name_bytes[10] = b'x';
    let bad = DefeatNote::Way {
        way: DefeatWay::RestorePoint,
        restore: Some(bad_place),
    };
    assert!(!bad.is_valid());
    assert!(DefeatNote::MenuChoice(GameOverChoice::Retry).is_valid());
}

#[test]
fn the_guard_disarms_the_flight_defeat_timer_of_the_partner_body() {
    let body = |state, flight_defeat_timer, health| PartnerBodyReading {
        state,
        flight_defeat_timer,
        health,
    };
    assert_eq!(
        partner_body_guard_act(body(0x91, 119.0, 50)),
        PartnerBodyGuardAct::DisarmTheFlightDefeatTimer
    );
    assert_eq!(
        partner_body_guard_act(body(0x91, 0.0, 50)),
        PartnerBodyGuardAct::Nothing
    );
    assert_eq!(
        partner_body_guard_act(body(0xC5, 0.0, 0)),
        PartnerBodyGuardAct::KeepTheHealthAtOne
    );
    assert_eq!(
        partner_body_guard_act(body(0xC5, 0.0, 1)),
        PartnerBodyGuardAct::Nothing
    );
    assert_eq!(
        partner_body_guard_act(body(0x10, 50.0, 0)),
        PartnerBodyGuardAct::Nothing
    );
    let acted = body(0x91, 119.0, 50);
    let line =
        partner_body_guard_line(PartnerBodyGuardAct::DisarmTheFlightDefeatTimer, acted, true)
            .expect("a line when the guard acts");
    assert!(line.contains("0x91"));
    assert_eq!(
        partner_body_guard_line(PartnerBodyGuardAct::Nothing, acted, true),
        None
    );
}
