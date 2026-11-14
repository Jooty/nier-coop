use automata_game::event_follow::{
    EventCall, EventId, EventMachine, EventReadings, EventSend, EventStep, SkipPrompt,
};
use automata_game::load_gate::StoryPlace;
use automata_game::travel::JumpRole;

fn event(id: i32) -> EventId {
    EventId {
        kind: 0,
        id,
        sub: -1,
    }
}

fn readings(now_ms: u64) -> EventReadings {
    EventReadings {
        now_ms,
        role: JumpRole::Host,
        partner_present: true,
        local_in_play: true,
        partner_in_play: true,
        current_phase_id: 0x100,
        current_place: StoryPlace::new(0x100, "A"),
        partner_place: StoryPlace::new(0x100, "A"),
        partner_playing: None,
        slots: Vec::new(),
        event_flags_say_playing: true,
        started_heard: Vec::new(),
        ended_heard: Vec::new(),
        asks: Vec::new(),
        partner_report: None,
        local_unattended: None,
        skip_asks_heard: Vec::new(),
        local_event_pause_open: false,
        pauses_heard: Vec::new(),
        progress: None,
    }
}

fn playing_machine(pause_on: bool) -> EventMachine {
    let mut machine = EventMachine::new(true).with_event_pause(pause_on);
    machine.step(&readings(0));
    machine.on_play_request(event(1), 10);
    machine.on_start(event(1), 20);
    machine.step(&readings(30));
    machine
}

fn pause_sends(step: &EventStep) -> Vec<EventSend> {
    step.sends
        .iter()
        .copied()
        .filter(|send| matches!(send, EventSend::Pause { .. }))
        .collect()
}

fn partner_pauses(machine: &mut EventMachine, id: EventId, paused: bool, now_ms: u64) -> EventStep {
    machine.step(&EventReadings {
        pauses_heard: vec![(id, paused)],
        ..readings(now_ms)
    })
}

#[test]
fn an_own_event_pause_is_told_once_when_it_opens_and_once_when_it_closes() {
    let mut machine = playing_machine(true);
    let open = || EventReadings {
        local_event_pause_open: true,
        ..readings(0)
    };
    let opened = machine.step(&EventReadings {
        now_ms: 100,
        ..open()
    });
    assert_eq!(
        pause_sends(&opened),
        vec![EventSend::Pause {
            id: event(1),
            paused: true
        }]
    );
    let still = machine.step(&EventReadings {
        now_ms: 200,
        ..open()
    });
    assert!(pause_sends(&still).is_empty());
    let closed = machine.step(&readings(300));
    assert_eq!(
        pause_sends(&closed),
        vec![EventSend::Pause {
            id: event(1),
            paused: false
        }]
    );
    assert!(pause_sends(&machine.step(&readings(400))).is_empty());
}

#[test]
fn the_partners_pause_stops_the_clock_once_and_shows_the_note() {
    let mut machine = playing_machine(true);
    let paused = partner_pauses(&mut machine, event(1), true, 100);
    assert_eq!(paused.calls, vec![EventCall::PauseClock]);
    assert_eq!(
        paused.skip_prompt,
        Some(SkipPrompt::PartnerPausedTheCutscene)
    );
    let later = machine.step(&readings(5_000));
    assert!(later.calls.is_empty());
    assert_eq!(
        later.skip_prompt,
        Some(SkipPrompt::PartnerPausedTheCutscene)
    );
}

#[test]
fn the_note_names_the_partner_and_the_pause() {
    assert_eq!(
        SkipPrompt::PartnerPausedTheCutscene.text("Nine"),
        "Nine paused the cutscene"
    );
}

#[test]
fn the_partners_resume_resumes_the_clock_once() {
    let mut machine = playing_machine(true);
    partner_pauses(&mut machine, event(1), true, 100);
    let resumed = partner_pauses(&mut machine, event(1), false, 200);
    assert_eq!(resumed.calls, vec![EventCall::ResumeClock]);
    assert_eq!(resumed.skip_prompt, None);
    assert!(machine.step(&readings(300)).calls.is_empty());
}

#[test]
fn a_partner_pause_holds_with_no_time_limit() {
    let mut machine = playing_machine(true);
    partner_pauses(&mut machine, event(1), true, 100);
    for now_ms in [200, 120_101, 3_600_000, 86_400_000] {
        let step = machine.step(&readings(now_ms));
        assert!(step.calls.is_empty());
        assert_eq!(step.skip_prompt, Some(SkipPrompt::PartnerPausedTheCutscene));
    }
    let resumed = partner_pauses(&mut machine, event(1), false, 90_000_000);
    assert_eq!(resumed.calls, vec![EventCall::ResumeClock]);
}

#[test]
fn a_partner_pause_is_released_when_the_event_ends_here() {
    let mut machine = playing_machine(true);
    partner_pauses(&mut machine, event(1), true, 100);
    machine.on_end(event(1), false, 150);
    let step = machine.step(&readings(200));
    assert_eq!(step.calls, vec![EventCall::ResumeClock]);
}

#[test]
fn a_partner_pause_is_released_when_the_link_drops() {
    let mut machine = playing_machine(true);
    partner_pauses(&mut machine, event(1), true, 100);
    let down = machine.step(&EventReadings {
        partner_present: false,
        ..readings(200)
    });
    assert_eq!(down.calls, vec![EventCall::ResumeClock]);
    assert_eq!(down.skip_prompt, None);
}

#[test]
fn a_partner_pause_for_another_event_does_not_pause_this_one() {
    let mut machine = playing_machine(true);
    let step = partner_pauses(&mut machine, event(2), true, 100);
    assert!(step.calls.is_empty());
    assert_eq!(step.skip_prompt, None);
}

#[test]
fn an_own_open_pause_takes_no_call_and_the_hold_comes_back_when_it_closes() {
    let mut machine = playing_machine(true);
    let open = EventReadings {
        local_event_pause_open: true,
        ..readings(0)
    };
    machine.step(&EventReadings {
        now_ms: 100,
        ..open.clone()
    });
    let heard = machine.step(&EventReadings {
        now_ms: 200,
        pauses_heard: vec![(event(1), true)],
        ..open.clone()
    });
    assert!(heard.calls.is_empty());
    let closed = machine.step(&readings(300));
    assert_eq!(closed.calls, vec![EventCall::PauseClock]);
    assert!(machine.step(&readings(400)).calls.is_empty());
}

#[test]
fn the_partner_resuming_while_the_own_pause_is_open_makes_no_call() {
    let mut machine = playing_machine(true);
    partner_pauses(&mut machine, event(1), true, 100);
    let open = EventReadings {
        local_event_pause_open: true,
        ..readings(200)
    };
    machine.step(&open);
    let resumed = machine.step(&EventReadings {
        now_ms: 300,
        pauses_heard: vec![(event(1), false)],
        ..open
    });
    assert!(resumed.calls.is_empty());
    assert!(machine.step(&readings(400)).calls.is_empty());
}

#[test]
fn with_the_setting_off_nothing_is_told_or_called() {
    let mut machine = playing_machine(false);
    let step = EventReadings {
        local_event_pause_open: true,
        pauses_heard: vec![(event(1), true)],
        ..readings(100)
    };
    let result = machine.step(&step);
    assert!(pause_sends(&result).is_empty());
    assert!(result.calls.is_empty());
    assert_eq!(result.skip_prompt, None);
}

#[test]
fn outside_the_story_rule_a_pause_stays_the_copys_own() {
    let mut machine = playing_machine(true);
    let step = machine.step(&EventReadings {
        local_event_pause_open: true,
        partner_in_play: false,
        pauses_heard: vec![(event(1), true)],
        ..readings(100)
    });
    assert!(pause_sends(&step).is_empty());
    assert!(step.calls.is_empty());
}

fn has_line(step: &EventStep, text: &str) -> bool {
    step.logs.iter().any(|line| line.contains(text))
}

#[test]
fn each_release_of_a_partner_pause_logs_its_reason() {
    let mut ended = playing_machine(true);
    partner_pauses(&mut ended, event(1), true, 100);
    ended.on_end(event(1), false, 150);
    let step = ended.step(&readings(200));
    assert!(has_line(&step, "the event ended here"));

    let mut other = playing_machine(true);
    partner_pauses(&mut other, event(1), true, 100);
    other.on_end(event(1), false, 150);
    other.on_play_request(event(2), 160);
    other.on_start(event(2), 170);
    let step = other.step(&readings(200));
    assert_eq!(step.calls, vec![EventCall::ResumeClock]);

    let mut down = playing_machine(true);
    partner_pauses(&mut down, event(1), true, 100);
    let step = down.step(&EventReadings {
        partner_present: false,
        ..readings(200)
    });
    assert!(has_line(&step, "the link is down or a copy is out of play"));
}

#[test]
fn a_partner_pause_heard_before_the_event_plays_here_applies_when_it_plays() {
    let mut machine = EventMachine::new(true).with_event_pause(true);
    machine.step(&readings(0));
    let early = partner_pauses(&mut machine, event(1), true, 10);
    assert!(early.calls.is_empty());
    assert_eq!(early.skip_prompt, None);
    machine.on_play_request(event(1), 20);
    machine.on_start(event(1), 30);
    let playing = machine.step(&readings(40));
    assert_eq!(playing.calls, vec![EventCall::PauseClock]);
}

#[test]
fn a_hold_ends_on_the_first_tick_a_resume_could_be_dropped() {
    let mut machine = playing_machine(true);
    partner_pauses(&mut machine, event(1), true, 100);
    let unarmed = machine.step(&EventReadings {
        partner_present: false,
        ..readings(200)
    });
    assert_eq!(unarmed.calls, vec![EventCall::ResumeClock]);
    let back = machine.step(&readings(300));
    assert!(back.calls.is_empty());
    assert_eq!(back.skip_prompt, None);
}
