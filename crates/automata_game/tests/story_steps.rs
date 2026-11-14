use std::collections::BTreeSet;

use automata_game::exe::ExeVariant;
use automata_game::load_gate::StoryPlace;
use automata_game::story_steps::{
    ActionLook, Basis, ObservedDone, STORY_STEP_ASK_RESEND_MS, STORY_STEP_GIVE_UP_MS,
    STORY_STEP_HOST_NOT_STARTED_WAIT_MS, SetDone, SetDoneWhy, StepActionState, StepAnswer,
    StepClassLookup, StepClassRow, StepSend, StepSort, StepVerdict, StepsMachine, StepsReadings,
    StepsStep, StoryStepAnswer, StoryStepAsk, StoryStepDone, StoryStepsMode,
    WATCHED_STEP_COMPARE_AFTER_MS, step_class_table, step_class_table_rows_unread,
};
use automata_game::travel::JumpRole;

const STEP_ID: u32 = 0x1234;
const OTHER_STEP_ID: u32 = 0x5678;
const OWN_VALUE: i32 = 3;
const HOST_VALUE: i32 = 7;

fn row(name: &str) -> &'static StepClassRow {
    step_class_table()
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("{name} is in the table"))
}

fn place(name: &str) -> StoryPlace {
    StoryPlace::new(0x100, name)
}

fn done_of(id: u32, class: &str, value: i32) -> ObservedDone {
    let row = row(class);
    ObservedDone {
        id,
        class: Some(row),
        vftable_rva: row.vftable.main,
        value,
        ours: false,
    }
}

fn story_done(id: u32) -> ObservedDone {
    done_of(id, "ConditionAction", OWN_VALUE)
}

fn ours(mut done: ObservedDone) -> ObservedDone {
    done.ours = true;
    done
}

fn readings(role: JumpRole, now_ms: u64) -> StepsReadings {
    StepsReadings {
        now_ms,
        role,
        partner_present: true,
        place: place("A"),
        partner_place: place("A"),
        ..StepsReadings::default()
    }
}

fn machine_as(mode: StoryStepsMode, role: JumpRole) -> StepsMachine {
    let mut machine = StepsMachine::new(mode);
    machine.step(&readings(role, 0));
    machine
}

fn host_done(number: u32, id: u32) -> StoryStepDone {
    StoryStepDone {
        number,
        id,
        value: HOST_VALUE,
        place: place("A"),
    }
}

fn look(id: u32, state: StepActionState) -> ActionLook {
    ActionLook {
        id,
        state: Some(state),
        value: HOST_VALUE,
    }
}

fn asks(step: &StepsStep) -> Vec<StoryStepAsk> {
    step.sends
        .iter()
        .filter_map(|send| match send {
            StepSend::Ask(ask) => Some(ask.clone()),
            _ => None,
        })
        .collect()
}

fn tells(step: &StepsStep) -> Vec<StoryStepDone> {
    step.sends
        .iter()
        .filter_map(|send| match send {
            StepSend::Done(done) => Some(done.clone()),
            _ => None,
        })
        .collect()
}

fn answers(step: &StepsStep) -> Vec<StoryStepAnswer> {
    step.sends
        .iter()
        .filter_map(|send| match send {
            StepSend::Answer(answer) => Some(*answer),
            _ => None,
        })
        .collect()
}

fn has_line(step: &StepsStep, part: &str) -> bool {
    step.logs.iter().any(|line| line.contains(part))
}

fn guest_holding(id: u32) -> StepsMachine {
    let mut machine = machine_as(StoryStepsMode::On, JumpRole::Guest);
    assert_eq!(
        machine.on_own_done(&story_done(id), 100),
        StepVerdict::Refuse
    );
    machine
}

#[test]
fn watch_is_the_default_mode_and_every_word_reads() {
    assert_eq!(StoryStepsMode::default(), StoryStepsMode::Watch);
    for mode in [
        StoryStepsMode::On,
        StoryStepsMode::Watch,
        StoryStepsMode::Off,
    ] {
        assert_eq!(StoryStepsMode::from_text(mode.label()), Some(mode));
    }
    assert_eq!(StoryStepsMode::from_text("sometimes"), None);
    assert!(!StoryStepsMode::Off.installs_hooks());
    assert!(StoryStepsMode::Watch.installs_hooks());
    assert!(!StoryStepsMode::Watch.holds_and_applies());
    assert!(StoryStepsMode::On.holds_and_applies());
}

#[test]
fn every_line_of_the_class_table_reads_as_a_row() {
    assert!(step_class_table_rows_unread().is_empty());
    assert!(step_class_table().len() >= 160);
}

#[test]
fn no_two_rows_share_a_class_or_a_vftable() {
    let table = step_class_table();
    let names: BTreeSet<&str> = table.iter().map(|row| row.name).collect();
    let main: BTreeSet<u32> = table.iter().map(|row| row.vftable.main).collect();
    let compat: BTreeSet<u32> = table.iter().map(|row| row.vftable.compat).collect();
    assert_eq!(names.len(), table.len());
    assert_eq!(main.len(), table.len());
    assert_eq!(compat.len(), table.len());
}

#[test]
fn the_classes_other_stages_lead_are_never_mirrored() {
    for (name, sort) in [
        ("PhaseJumpAction", StepSort::LedByPhaseFollow),
        ("SubPhaseAction", StepSort::LedByPhaseFollow),
        ("SubphaseChangeAction", StepSort::LedByPhaseFollow),
        ("PhaseEventAction", StepSort::LedByEventFollow),
        ("EnemyGenerator", StepSort::LedByGeneratorCounts),
        ("tr::BattleCounter", StepSort::LedByGeneratorCounts),
    ] {
        assert_eq!(row(name).sort, sort, "{name}");
        assert!(!sort.is_mirrored(), "{name}");
    }
}

#[test]
fn one_player_and_presentation_classes_are_never_mirrored() {
    for name in [
        "TalkMan",
        "HapShopLocationAction",
        "Tutorial",
        "PositionJump",
    ] {
        assert_eq!(row(name).sort, StepSort::OnePlayer, "{name}");
    }
    for name in ["BgmAction", "CameraAction", "FadeAction", "RouteAction"] {
        assert_eq!(row(name).sort, StepSort::Presentation, "{name}");
    }
    assert!(!StepSort::OnePlayer.is_mirrored());
    assert!(!StepSort::Presentation.is_mirrored());
}

#[test]
fn story_and_area_classes_are_mirrored() {
    for name in [
        "ConditionAction",
        "DelayAction",
        "SignalAction",
        "ScriptAction",
    ] {
        assert_eq!(row(name).sort, StepSort::Story, "{name}");
    }
    for name in ["AreaAction", "QuestArea"] {
        assert_eq!(row(name).sort, StepSort::Area, "{name}");
    }
    assert!(StepSort::Story.is_mirrored());
    assert!(StepSort::Area.is_mirrored());
}

#[test]
fn every_row_has_a_reason_and_only_the_led_classes_are_proven() {
    for row in step_class_table() {
        assert!(!row.reason.is_empty(), "{}", row.name);
        if row.basis == Basis::Proven {
            assert!(!row.sort.is_mirrored(), "{}", row.name);
        }
    }
}

#[test]
fn a_class_is_found_by_the_vftable_of_either_exe() {
    let condition = row("ConditionAction");
    let main = StepClassLookup::for_variant(ExeVariant::Main);
    let compat = StepClassLookup::for_variant(ExeVariant::Compat);
    assert_eq!(main.len(), step_class_table().len());
    assert_eq!(main.row_of_vftable_rva(0xC4_D410), Some(condition));
    assert_eq!(compat.row_of_vftable_rva(0xC4_33D0), Some(condition));
    assert_eq!(main.row_of_vftable_rva(0xC4_33D0), None);
    assert_eq!(main.row_of_vftable_rva(0x10), None);
}

#[test]
fn off_and_solo_act_on_nothing() {
    let mut off = machine_as(StoryStepsMode::Off, JumpRole::Guest);
    assert!(!off.is_acting());
    assert_eq!(off.on_own_done(&story_done(STEP_ID), 10), StepVerdict::Pass);
    assert_eq!(
        off.step(&readings(JumpRole::Guest, 20)),
        StepsStep::default()
    );
    let mut solo = machine_as(StoryStepsMode::On, JumpRole::Solo);
    assert!(!solo.is_acting());
    assert_eq!(
        solo.on_own_done(&story_done(STEP_ID), 10),
        StepVerdict::Pass
    );
    let mut alone = StepsMachine::new(StoryStepsMode::On);
    let mut no_partner = readings(JumpRole::Guest, 0);
    no_partner.partner_present = false;
    alone.step(&no_partner);
    assert_eq!(
        alone.on_own_done(&story_done(STEP_ID), 10),
        StepVerdict::Pass
    );
}

#[test]
fn the_host_tells_its_mirrored_dones_by_id_in_order_in_both_modes() {
    for mode in [StoryStepsMode::On, StoryStepsMode::Watch] {
        let mut host = machine_as(mode, JumpRole::Host);
        assert_eq!(
            host.on_own_done(&story_done(STEP_ID), 10),
            StepVerdict::Pass
        );
        assert_eq!(
            host.on_own_done(&done_of(OTHER_STEP_ID, "AreaAction", 1), 11),
            StepVerdict::Pass
        );
        let step = host.step(&readings(JumpRole::Host, 20));
        let told = tells(&step);
        assert_eq!(told.len(), 2);
        assert_eq!((told[0].number, told[0].id), (1, STEP_ID));
        assert_eq!((told[1].number, told[1].id), (2, OTHER_STEP_ID));
        assert_eq!(told[0].place, place("A"));
        assert!(has_line(
            &step,
            "host done id 0x00001234 class ConditionAction (story)"
        ));
    }
}

#[test]
fn dones_that_are_not_mirrored_pass_untold_with_a_line() {
    let mut host = machine_as(StoryStepsMode::On, JumpRole::Host);
    assert_eq!(
        host.on_own_done(&done_of(STEP_ID, "TalkMan", 1), 10),
        StepVerdict::Pass
    );
    let step = host.step(&readings(JumpRole::Host, 20));
    assert!(tells(&step).is_empty());
    assert!(has_line(&step, "class TalkMan (one_player) not mirrored"));
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    assert_eq!(
        guest.on_own_done(&done_of(STEP_ID, "PhaseJumpAction", 1), 10),
        StepVerdict::Pass
    );
}

#[test]
fn a_class_not_in_the_table_passes_and_is_named_once() {
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    let unknown = ObservedDone {
        id: STEP_ID,
        class: None,
        vftable_rva: 0xC9_0000,
        value: 0,
        ours: false,
    };
    assert_eq!(guest.on_own_done(&unknown, 10), StepVerdict::Pass);
    assert_eq!(guest.on_own_done(&unknown, 11), StepVerdict::Pass);
    let step = guest.step(&readings(JumpRole::Guest, 20));
    let named = step
        .logs
        .iter()
        .filter(|line| line.contains("vftable rva 0xC90000"))
        .count();
    assert_eq!(named, 1);
}

#[test]
fn the_guests_own_done_is_held_and_asked_of_the_host() {
    let mut guest = guest_holding(STEP_ID);
    let step = guest.step(&readings(JumpRole::Guest, 150));
    assert!(has_line(&step, "held this game's done of id 0x00001234"));
    assert_eq!(
        asks(&step),
        vec![StoryStepAsk {
            id: STEP_ID,
            value: OWN_VALUE,
            place: place("A"),
        }]
    );
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 160),
        StepVerdict::Refuse
    );
}

#[test]
fn the_ask_is_sent_again_every_resend_interval() {
    let mut guest = guest_holding(STEP_ID);
    assert_eq!(asks(&guest.step(&readings(JumpRole::Guest, 200))).len(), 1);
    let soon = 200 + STORY_STEP_ASK_RESEND_MS - 1;
    assert!(asks(&guest.step(&readings(JumpRole::Guest, soon))).is_empty());
    let later = 200 + STORY_STEP_ASK_RESEND_MS;
    assert_eq!(
        asks(&guest.step(&readings(JumpRole::Guest, later))).len(),
        1
    );
}

#[test]
fn the_host_sets_a_running_asked_action_done_and_keeps_it_done() {
    let mut host = machine_as(StoryStepsMode::On, JumpRole::Host);
    let ask = StoryStepAsk {
        id: STEP_ID,
        value: OWN_VALUE,
        place: place("A"),
    };
    let mut asked = readings(JumpRole::Host, 10);
    asked.asks = vec![ask];
    asked.looks = vec![look(STEP_ID, StepActionState::Running)];
    let step = host.step(&asked);
    assert_eq!(
        step.set_done,
        vec![SetDone {
            id: STEP_ID,
            value: OWN_VALUE,
            why: SetDoneWhy::AskedByTheGuest,
        }]
    );
    assert_eq!(
        host.on_own_done(&ours(story_done(STEP_ID)), 11),
        StepVerdict::Pass
    );
    assert_eq!(
        tells(&host.step(&readings(JumpRole::Host, 20)))[0].id,
        STEP_ID
    );
    assert_eq!(host.on_leave_done(STEP_ID), StepVerdict::Refuse);
}

#[test]
fn the_host_answers_not_started_or_another_place_or_tells_its_done_again() {
    let mut host = machine_as(StoryStepsMode::On, JumpRole::Host);
    let ask = |id: u32, at: &str| StoryStepAsk {
        id,
        value: OWN_VALUE,
        place: place(at),
    };
    let mut asked = readings(JumpRole::Host, 10);
    asked.asks = vec![ask(STEP_ID, "A"), ask(OTHER_STEP_ID, "B")];
    asked.looks = vec![look(STEP_ID, StepActionState::NotStarted)];
    let step = host.step(&asked);
    assert_eq!(
        answers(&step),
        vec![
            StoryStepAnswer {
                id: STEP_ID,
                answer: StepAnswer::NotStarted,
            },
            StoryStepAnswer {
                id: OTHER_STEP_ID,
                answer: StepAnswer::OtherPlace,
            },
        ]
    );
    host.on_own_done(&story_done(STEP_ID), 20);
    host.step(&readings(JumpRole::Host, 30));
    let mut again = readings(JumpRole::Host, 40);
    again.asks = vec![ask(STEP_ID, "A")];
    let told = tells(&host.step(&again));
    assert_eq!(told.len(), 1);
    assert_eq!((told[0].number, told[0].id), (1, STEP_ID));
    let mut done_in_game = readings(JumpRole::Host, 50);
    done_in_game.asks = vec![ask(OTHER_STEP_ID, "A")];
    done_in_game.looks = vec![look(OTHER_STEP_ID, StepActionState::Done)];
    let told = tells(&host.step(&done_in_game));
    assert_eq!((told[0].id, told[0].value), (OTHER_STEP_ID, HOST_VALUE));
}

#[test]
fn the_host_in_watch_answers_no_ask() {
    let mut host = machine_as(StoryStepsMode::Watch, JumpRole::Host);
    let mut asked = readings(JumpRole::Host, 10);
    asked.asks = vec![StoryStepAsk {
        id: STEP_ID,
        value: OWN_VALUE,
        place: place("A"),
    }];
    asked.looks = vec![look(STEP_ID, StepActionState::Running)];
    let step = host.step(&asked);
    assert!(step.sends.is_empty());
    assert!(step.set_done.is_empty());
}

#[test]
fn a_held_done_takes_the_hosts_done_and_then_stays_done() {
    let mut guest = guest_holding(STEP_ID);
    guest.step(&readings(JumpRole::Guest, 150));
    let mut heard = readings(JumpRole::Guest, 400);
    heard.told = vec![host_done(1, STEP_ID)];
    heard.looks = vec![look(STEP_ID, StepActionState::Running)];
    let step = guest.step(&heard);
    assert_eq!(
        step.set_done,
        vec![SetDone {
            id: STEP_ID,
            value: HOST_VALUE,
            why: SetDoneWhy::HostsDone,
        }]
    );
    assert!(has_line(&step, "this game's own was held 300 ms"));
    assert_eq!(
        guest.on_own_done(&ours(story_done(STEP_ID)), 401),
        StepVerdict::Pass
    );
    assert_eq!(guest.on_leave_done(STEP_ID), StepVerdict::Refuse);
    assert_eq!(guest.on_leave_done(OTHER_STEP_ID), StepVerdict::Pass);
}

#[test]
fn the_hosts_done_waits_until_the_guests_action_has_started() {
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    let mut heard = readings(JumpRole::Guest, 10);
    heard.told = vec![host_done(1, STEP_ID)];
    heard.looks = vec![look(STEP_ID, StepActionState::NotStarted)];
    assert!(guest.step(&heard).set_done.is_empty());
    assert_eq!(guest.ids_to_look_at(), vec![STEP_ID]);
    let mut not_loaded = readings(JumpRole::Guest, 20);
    not_loaded.looks = vec![ActionLook {
        id: STEP_ID,
        state: None,
        value: 0,
    }];
    assert!(guest.step(&not_loaded).set_done.is_empty());
    let mut started = readings(JumpRole::Guest, 30);
    started.looks = vec![look(STEP_ID, StepActionState::Running)];
    assert_eq!(guest.step(&started).set_done.len(), 1);
    assert!(guest.ids_to_look_at().is_empty());
}

#[test]
fn the_guests_own_done_passes_when_the_hosts_is_already_known() {
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    let mut heard = readings(JumpRole::Guest, 10);
    heard.told = vec![host_done(1, STEP_ID)];
    guest.step(&heard);
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 20),
        StepVerdict::Pass
    );
    assert_eq!(guest.on_leave_done(STEP_ID), StepVerdict::Refuse);
}

#[test]
fn a_hosts_done_already_done_here_is_only_logged() {
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    let mut heard = readings(JumpRole::Guest, 10);
    heard.told = vec![host_done(1, STEP_ID)];
    heard.looks = vec![look(STEP_ID, StepActionState::Done)];
    let step = guest.step(&heard);
    assert!(step.set_done.is_empty());
    assert!(has_line(&step, "already done here"));
}

#[test]
fn without_an_answer_the_held_done_passes_after_the_give_up_time() {
    let mut guest = guest_holding(STEP_ID);
    let before = 100 + STORY_STEP_GIVE_UP_MS - 1;
    assert!(
        guest
            .step(&readings(JumpRole::Guest, before))
            .set_done
            .is_empty()
    );
    let step = guest.step(&readings(JumpRole::Guest, 100 + STORY_STEP_GIVE_UP_MS));
    assert_eq!(
        step.set_done,
        vec![SetDone {
            id: STEP_ID,
            value: OWN_VALUE,
            why: SetDoneWhy::GaveUpWaiting,
        }]
    );
    assert!(has_line(&step, "GAVE UP on id 0x00001234"));
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 100 + STORY_STEP_GIVE_UP_MS + 1),
        StepVerdict::Pass
    );
}

#[test]
fn after_a_not_started_answer_the_held_done_passes_sooner() {
    let mut guest = guest_holding(STEP_ID);
    let mut answered = readings(JumpRole::Guest, 500);
    answered.answers = vec![StoryStepAnswer {
        id: STEP_ID,
        answer: StepAnswer::NotStarted,
    }];
    assert!(guest.step(&answered).set_done.is_empty());
    let before = 500 + STORY_STEP_HOST_NOT_STARTED_WAIT_MS - 1;
    assert!(
        guest
            .step(&readings(JumpRole::Guest, before))
            .set_done
            .is_empty()
    );
    let at = 500 + STORY_STEP_HOST_NOT_STARTED_WAIT_MS;
    let step = guest.step(&readings(JumpRole::Guest, at));
    assert_eq!(step.set_done.len(), 1);
    assert!(has_line(&step, "the host's action had not started"));
}

#[test]
fn when_the_link_goes_down_every_held_done_passes() {
    let mut guest = guest_holding(STEP_ID);
    guest.on_own_done(&story_done(OTHER_STEP_ID), 110);
    let mut gone = readings(JumpRole::Guest, 200);
    gone.partner_present = false;
    let step = guest.step(&gone);
    let ids: Vec<u32> = step.set_done.iter().map(|set| set.id).collect();
    assert_eq!(ids, vec![STEP_ID, OTHER_STEP_ID]);
    assert!(
        step.set_done
            .iter()
            .all(|set| set.why == SetDoneWhy::LinkLost)
    );
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 210),
        StepVerdict::Pass
    );
}

#[test]
fn the_lists_are_emptied_at_a_phase_or_sub_phase_change() {
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    let mut heard = readings(JumpRole::Guest, 10);
    heard.told = vec![host_done(1, STEP_ID)];
    heard.looks = vec![look(STEP_ID, StepActionState::Running)];
    guest.step(&heard);
    assert_eq!(guest.on_leave_done(STEP_ID), StepVerdict::Refuse);
    let mut moved = readings(JumpRole::Guest, 20);
    moved.place = place("B");
    moved.partner_place = place("B");
    let step = guest.step(&moved);
    assert!(has_line(
        &step,
        "lists emptied (the phase or sub-phase changed)"
    ));
    assert_eq!(guest.on_leave_done(STEP_ID), StepVerdict::Pass);
}

#[test]
fn a_reset_of_an_action_forgets_its_id() {
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    let mut heard = readings(JumpRole::Guest, 10);
    heard.told = vec![host_done(1, STEP_ID)];
    heard.looks = vec![look(STEP_ID, StepActionState::Running)];
    guest.step(&heard);
    guest.on_reset(STEP_ID);
    assert_eq!(guest.on_leave_done(STEP_ID), StepVerdict::Pass);
    assert!(has_line(
        &guest.step(&readings(JumpRole::Guest, 20)),
        "id 0x00001234 was reset here, forgotten"
    ));
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 30),
        StepVerdict::Refuse
    );
}

#[test]
fn a_guest_at_another_place_than_the_host_holds_nothing() {
    let mut guest = StepsMachine::new(StoryStepsMode::On);
    let mut apart = readings(JumpRole::Guest, 0);
    apart.partner_place = place("B");
    guest.step(&apart);
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 10),
        StepVerdict::Pass
    );
    assert!(has_line(&guest.step(&apart), "the host is at 0x100 B"));
}

#[test]
fn a_hosts_done_for_the_next_place_is_kept_until_the_guest_is_there() {
    let mut guest = machine_as(StoryStepsMode::On, JumpRole::Guest);
    let mut early = readings(JumpRole::Guest, 10);
    early.told = vec![StoryStepDone {
        place: place("B"),
        ..host_done(1, STEP_ID)
    }];
    let step = guest.step(&early);
    assert!(has_line(&step, "kept until this game is there"));
    let mut there = readings(JumpRole::Guest, 20);
    there.place = place("B");
    there.partner_place = place("B");
    guest.step(&there);
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 30),
        StepVerdict::Pass
    );
}

#[test]
fn watch_never_holds_never_asks_and_never_sets_a_done() {
    let mut guest = machine_as(StoryStepsMode::Watch, JumpRole::Guest);
    assert_eq!(
        guest.on_own_done(&story_done(STEP_ID), 10),
        StepVerdict::Pass
    );
    assert_eq!(guest.on_leave_done(STEP_ID), StepVerdict::Pass);
    let mut heard = readings(JumpRole::Guest, 20);
    heard.told = vec![host_done(1, OTHER_STEP_ID)];
    heard.looks = vec![look(OTHER_STEP_ID, StepActionState::Running)];
    for now_ms in [20, 30_000, 60_000] {
        heard.now_ms = now_ms;
        let step = guest.step(&heard);
        assert!(step.sends.is_empty());
        assert!(step.set_done.is_empty());
    }
    assert!(guest.ids_to_look_at().is_empty());
}

#[test]
fn watch_says_how_long_on_would_have_held_an_early_own_done() {
    let mut guest = machine_as(StoryStepsMode::Watch, JumpRole::Guest);
    guest.on_own_done(&story_done(STEP_ID), 1_000);
    let mut heard = readings(JumpRole::Guest, 1_750);
    heard.told = vec![host_done(1, STEP_ID)];
    let step = guest.step(&heard);
    assert!(has_line(
        &step,
        "watch: this game's done of id 0x00001234 (ConditionAction) came 750 ms before the host's: on would have held it 750 ms"
    ));
    assert!(has_line(
        &step,
        "guest done id 0x00001234 class ConditionAction (story)"
    ));
}

#[test]
fn watch_says_when_the_own_done_came_after_the_hosts() {
    let mut guest = machine_as(StoryStepsMode::Watch, JumpRole::Guest);
    let mut heard = readings(JumpRole::Guest, 1_000);
    heard.told = vec![host_done(1, STEP_ID)];
    guest.step(&heard);
    guest.on_own_done(&story_done(STEP_ID), 1_400);
    assert!(has_line(
        &guest.step(&readings(JumpRole::Guest, 1_500)),
        "came 400 ms after the host's: on would not hold it"
    ));
}

#[test]
fn watch_names_a_host_done_this_game_has_not_reached_in_time() {
    let mut guest = machine_as(StoryStepsMode::Watch, JumpRole::Guest);
    let mut heard = readings(JumpRole::Guest, 1_000);
    heard.told = vec![host_done(1, STEP_ID)];
    guest.step(&heard);
    let before = 1_000 + WATCHED_STEP_COMPARE_AFTER_MS - 1;
    assert!(!has_line(
        &guest.step(&readings(JumpRole::Guest, before)),
        "not reached here"
    ));
    let at = 1_000 + WATCHED_STEP_COMPARE_AFTER_MS;
    let step = guest.step(&readings(JumpRole::Guest, at));
    assert!(has_line(
        &step,
        "the host's done of id 0x00001234 is not reached here within 10000 ms"
    ));
    assert!(!has_line(
        &guest.step(&readings(JumpRole::Guest, at + 1)),
        "not reached here"
    ));
}

#[test]
fn watch_names_an_own_done_the_host_never_matched() {
    let mut guest = machine_as(StoryStepsMode::Watch, JumpRole::Guest);
    guest.on_own_done(&story_done(STEP_ID), 1_000);
    let step = guest.step(&readings(JumpRole::Guest, 1_000 + STORY_STEP_GIVE_UP_MS));
    assert!(has_line(
        &step,
        "no host done for this game's done of id 0x00001234 (ConditionAction) within 20000 ms"
    ));
}

#[test]
fn the_install_line_names_the_campaign_default_only_when_it_applied() {
    use automata_game::story_steps::story_steps_on_line;
    let defaulted = story_steps_on_line(StoryStepsMode::On, 12, true);
    assert!(defaulted.contains("the default for a campaign since the watch run of 2026-11-14"));
    let chosen = story_steps_on_line(StoryStepsMode::On, 12, false);
    assert!(!chosen.contains("default for a campaign"));
}
