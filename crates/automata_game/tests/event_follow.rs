use automata_game::event_follow::{
    EVENT_ASK_RESEND_MS, EVENT_FOLLOW_GRACE_MS, EVENT_SLOT_LOAD_FAILED_STATE,
    EVENT_SLOT_LOADED_STATE, EVENT_SLOT_PLAYING_STATE, EVENT_SLOT_STRIDE, EVENT_SLOT_TABLE_BYTES,
    EVENT_STARTED_FOR_THE_PARTNER_LOAD_GIVE_UP_MS, EventAsk, EventAskWhat, EventCall, EventId,
    EventMachine, EventReadings, EventSend, EventSlot, EventStep,
    GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS, GUEST_EVENT_START_GIVE_UP_MS,
    GUEST_SKIP_ASK_GIVE_UP_MS, HOST_EVENT_START_REMEMBERED_MS, PlayVerdict, SkipVerdict,
    slots_of_table,
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

fn readings(role: JumpRole, now_ms: u64) -> EventReadings {
    EventReadings {
        now_ms,
        role,
        partner_present: true,
        local_in_play: true,
        partner_in_play: true,
        current_phase_id: 0x100,
        current_place: StoryPlace::new(0x100, "A"),
        partner_place: StoryPlace::new(0x100, "A"),
        partner_playing: None,
        slots: Vec::new(),
        event_flags_say_playing: false,
        started_heard: Vec::new(),
        ended_heard: Vec::new(),
        asks: Vec::new(),
    }
}

fn loaded(id: EventId) -> EventSlot {
    EventSlot {
        flags: 1 | 2,
        id,
        state: EVENT_SLOT_LOADED_STATE,
    }
}

fn host() -> EventMachine {
    let mut machine = EventMachine::new(true);
    machine.step(&readings(JumpRole::Host, 0));
    machine
}

fn guest() -> EventMachine {
    let mut machine = EventMachine::new(true);
    machine.step(&readings(JumpRole::Guest, 0));
    machine
}

fn guest_hears_start(machine: &mut EventMachine, id: EventId, now_ms: u64) -> EventStep {
    machine.step(&EventReadings {
        started_heard: vec![id],
        ..readings(JumpRole::Guest, now_ms)
    })
}

fn guest_hears_end(
    machine: &mut EventMachine,
    id: EventId,
    skipped: bool,
    now_ms: u64,
) -> EventStep {
    machine.step(&EventReadings {
        ended_heard: vec![(id, skipped)],
        ..readings(JumpRole::Guest, now_ms)
    })
}

fn host_hears_ask(machine: &mut EventMachine, ask: EventAsk, at: EventReadings) -> EventStep {
    machine.step(&EventReadings {
        asks: vec![ask],
        ..at
    })
}

fn start_ask(id: EventId) -> EventAsk {
    EventAsk {
        id,
        what: EventAskWhat::Start,
    }
}

fn skip_ask(id: EventId) -> EventAsk {
    EventAsk {
        id,
        what: EventAskWhat::Skip,
    }
}

fn guest_plays_with_the_host(machine: &mut EventMachine, id: EventId, now_ms: u64) {
    guest_hears_start(machine, id, now_ms);
    assert_eq!(machine.on_play_request(id, now_ms), PlayVerdict::Pass);
    machine.on_start(id, now_ms);
    machine.step(&readings(JumpRole::Guest, now_ms));
}

fn has_ask(step: &EventStep, ask: EventAsk) -> bool {
    step.sends.contains(&EventSend::Ask(ask))
}

#[test]
fn the_host_tells_its_own_story_start() {
    let mut machine = host();
    assert_eq!(machine.on_play_request(event(1), 10), PlayVerdict::Pass);
    machine.on_start(event(1), 20);
    let step = machine.step(&readings(JumpRole::Host, 30));
    assert_eq!(step.sends, vec![EventSend::Started(event(1))]);
    assert_eq!(step.playing_story_event, Some(event(1)));
}

#[test]
fn the_host_tells_its_own_end_with_skipped() {
    for skipped in [false, true] {
        let mut machine = host();
        machine.on_play_request(event(1), 10);
        machine.on_start(event(1), 20);
        machine.step(&readings(JumpRole::Host, 30));
        machine.on_end(event(1), skipped, 40);
        let step = machine.step(&readings(JumpRole::Host, 50));
        assert_eq!(
            step.sends,
            vec![EventSend::Ended {
                id: event(1),
                skipped
            }]
        );
        assert_eq!(step.playing_story_event, None);
    }
}

#[test]
fn an_event_the_story_script_did_not_ask_for_is_not_told() {
    let mut machine = host();
    machine.on_start(event(2), 20);
    machine.on_end(event(2), false, 30);
    let step = machine.step(&readings(JumpRole::Host, 40));
    assert!(step.sends.is_empty());
    assert_eq!(step.playing_story_event, None);
}

#[test]
fn the_guest_never_plays_an_event_before_the_host_started_it() {
    let mut machine = guest();
    assert_eq!(machine.on_play_request(event(1), 100), PlayVerdict::Hold);
    let step = machine.step(&readings(JumpRole::Guest, 110));
    assert!(has_ask(&step, start_ask(event(1))));
    assert!(!step.calls.contains(&EventCall::ReleaseHeldPlay(event(1))));
}

#[test]
fn the_guest_asks_again_every_resend_interval() {
    let mut machine = guest();
    machine.on_play_request(event(1), 100);
    assert!(has_ask(
        &machine.step(&readings(JumpRole::Guest, 100)),
        start_ask(event(1))
    ));
    let early = machine.step(&readings(JumpRole::Guest, 100 + EVENT_ASK_RESEND_MS - 1));
    assert!(early.sends.is_empty());
    let due = machine.step(&readings(JumpRole::Guest, 100 + EVENT_ASK_RESEND_MS));
    assert!(has_ask(&due, start_ask(event(1))));
    assert!(due.logs.iter().any(|line| line.contains("still no start")));
}

#[test]
fn a_held_play_is_released_when_the_host_starts_it() {
    let mut machine = guest();
    machine.on_play_request(event(1), 100);
    machine.step(&readings(JumpRole::Guest, 100));
    let step = guest_hears_start(&mut machine, event(1), 600);
    assert!(step.calls.contains(&EventCall::ReleaseHeldPlay(event(1))));
    let after = machine.step(&readings(JumpRole::Guest, 2_000));
    assert!(after.sends.is_empty() && after.calls.is_empty());
}

#[test]
fn a_held_play_is_released_when_the_hosts_state_shows_it() {
    let mut machine = guest();
    machine.on_play_request(event(1), 100);
    machine.step(&readings(JumpRole::Guest, 100));
    let step = machine.step(&EventReadings {
        partner_playing: Some(event(1)),
        ..readings(JumpRole::Guest, 400)
    });
    assert!(step.calls.contains(&EventCall::ReleaseHeldPlay(event(1))));
}

#[test]
fn the_guest_play_passes_when_the_host_started_it_recently() {
    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    guest_hears_end(&mut machine, event(1), false, 2_000);
    assert_eq!(
        machine.on_play_request(event(1), 1_000 + HOST_EVENT_START_REMEMBERED_MS),
        PlayVerdict::Pass
    );
}

#[test]
fn the_guest_play_is_held_when_the_hosts_start_is_long_past_and_ended() {
    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    guest_hears_end(&mut machine, event(1), false, 2_000);
    machine.step(&readings(JumpRole::Guest, 30_000));
    assert_eq!(machine.on_play_request(event(1), 30_000), PlayVerdict::Hold);
}

#[test]
fn the_guest_play_passes_while_the_host_plays_it() {
    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    machine.step(&readings(JumpRole::Guest, 60_000));
    assert_eq!(machine.on_play_request(event(1), 60_000), PlayVerdict::Pass);
}

#[test]
fn a_held_play_runs_here_with_a_loud_line_after_the_give_up() {
    let mut machine = guest();
    machine.on_play_request(event(1), 100);
    machine.step(&readings(JumpRole::Guest, 100));
    let before = machine.step(&readings(
        JumpRole::Guest,
        100 + GUEST_EVENT_START_GIVE_UP_MS - 1,
    ));
    assert!(!before.calls.contains(&EventCall::ReleaseHeldPlay(event(1))));
    let step = machine.step(&readings(
        JumpRole::Guest,
        100 + GUEST_EVENT_START_GIVE_UP_MS,
    ));
    assert!(step.calls.contains(&EventCall::ReleaseHeldPlay(event(1))));
    assert!(step.logs.iter().any(|line| line.contains("NO START OF")));
}

#[test]
fn the_guest_starts_the_hosts_event_by_its_data_id_after_the_grace() {
    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    let early = machine.step(&readings(
        JumpRole::Guest,
        1_000 + EVENT_FOLLOW_GRACE_MS - 1,
    ));
    assert!(early.calls.is_empty());
    let step = machine.step(&readings(JumpRole::Guest, 1_000 + EVENT_FOLLOW_GRACE_MS));
    assert_eq!(step.calls, vec![EventCall::Load(event(1))]);
    let played = machine.step(&EventReadings {
        slots: vec![loaded(event(1))],
        ..readings(JumpRole::Guest, 4_100)
    });
    assert_eq!(played.calls, vec![EventCall::Play(event(1))]);
    machine.on_start(event(1), 4_200);
    machine.on_end(event(1), false, 9_000);
    let released = machine.step(&readings(JumpRole::Guest, 9_010));
    assert!(released.calls.contains(&EventCall::Release(event(1))));
}

#[test]
fn the_guest_does_not_start_the_hosts_event_when_it_holds_or_plays_it() {
    let mut holding = guest();
    holding.on_play_request(event(1), 900);
    holding.step(&readings(JumpRole::Guest, 900));
    guest_hears_start(&mut holding, event(1), 1_000);
    let step = holding.step(&readings(JumpRole::Guest, 1_000 + EVENT_FOLLOW_GRACE_MS));
    assert!(!step.calls.contains(&EventCall::Load(event(1))));

    let mut playing = guest();
    guest_plays_with_the_host(&mut playing, event(1), 1_000);
    let step = playing.step(&readings(JumpRole::Guest, 1_000 + EVENT_FOLLOW_GRACE_MS));
    assert!(!step.calls.contains(&EventCall::Load(event(1))));
}

#[test]
fn the_guest_does_not_start_the_hosts_event_after_the_hosts_end() {
    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    guest_hears_end(&mut machine, event(1), false, 2_000);
    let step = machine.step(&readings(JumpRole::Guest, 1_000 + EVENT_FOLLOW_GRACE_MS));
    assert!(step.calls.is_empty());
}

#[test]
fn the_guest_waits_to_start_the_hosts_event_at_another_place_or_in_another_event() {
    let mut elsewhere = guest();
    guest_hears_start(&mut elsewhere, event(1), 1_000);
    let step = elsewhere.step(&EventReadings {
        partner_place: StoryPlace::new(0x100, "B"),
        ..readings(JumpRole::Guest, 1_000 + EVENT_FOLLOW_GRACE_MS)
    });
    assert!(step.calls.is_empty());
    assert!(step.logs.iter().any(|line| line.contains("another place")));

    let mut busy = guest();
    guest_hears_start(&mut busy, event(1), 1_000);
    let step = busy.step(&EventReadings {
        event_flags_say_playing: true,
        ..readings(JumpRole::Guest, 1_000 + EVENT_FOLLOW_GRACE_MS)
    });
    assert!(step.calls.is_empty());
}

#[test]
fn with_starts_for_the_partner_off_the_guest_never_starts_by_data_id() {
    let mut machine = EventMachine::new(false);
    machine.step(&readings(JumpRole::Guest, 0));
    guest_hears_start(&mut machine, event(1), 1_000);
    let step = machine.step(&readings(JumpRole::Guest, 10_000));
    assert!(step.calls.is_empty());
}

#[test]
fn an_event_started_for_the_partner_is_skipped_at_once_when_the_script_plays_it_later() {
    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    machine.step(&readings(JumpRole::Guest, 4_000));
    machine.step(&EventReadings {
        slots: vec![loaded(event(1))],
        ..readings(JumpRole::Guest, 4_100)
    });
    machine.on_start(event(1), 4_200);
    machine.on_end(event(1), false, 9_000);
    machine.step(&readings(JumpRole::Guest, 9_010));
    assert_eq!(machine.on_play_request(event(1), 20_000), PlayVerdict::Pass);
    machine.on_start(event(1), 20_010);
    let step = machine.step(&readings(JumpRole::Guest, 20_020));
    assert!(step.calls.contains(&EventCall::Skip));
    assert!(step.sends.is_empty());
}

#[test]
fn the_hosts_replay_of_an_event_it_started_for_the_guest_is_skipped_and_not_told() {
    let mut machine = host();
    host_hears_ask(
        &mut machine,
        start_ask(event(1)),
        readings(JumpRole::Host, 1_000),
    );
    machine.step(&EventReadings {
        slots: vec![loaded(event(1))],
        ..readings(JumpRole::Host, 1_100)
    });
    machine.on_start(event(1), 1_200);
    machine.on_end(event(1), false, 5_000);
    machine.step(&readings(JumpRole::Host, 5_010));
    machine.on_play_request(event(1), 8_000);
    machine.on_start(event(1), 8_010);
    machine.on_end(event(1), true, 8_100);
    let step = machine.step(&readings(JumpRole::Host, 8_200));
    assert!(step.calls.contains(&EventCall::Skip));
    assert!(step.sends.is_empty());
}

#[test]
fn a_remembered_event_is_only_remembered_for_its_story_place() {
    let mut machine = host();
    host_hears_ask(
        &mut machine,
        start_ask(event(1)),
        readings(JumpRole::Host, 1_000),
    );
    machine.step(&EventReadings {
        slots: vec![loaded(event(1))],
        ..readings(JumpRole::Host, 1_100)
    });
    machine.on_start(event(1), 1_200);
    machine.on_end(event(1), false, 5_000);
    let elsewhere = EventReadings {
        current_place: StoryPlace::new(0x200, "C"),
        partner_place: StoryPlace::new(0x200, "C"),
        ..readings(JumpRole::Host, 6_000)
    };
    machine.step(&elsewhere);
    machine.on_play_request(event(1), 7_000);
    machine.on_start(event(1), 7_010);
    let step = machine.step(&EventReadings {
        now_ms: 7_020,
        ..elsewhere
    });
    assert!(!step.calls.contains(&EventCall::Skip));
    assert_eq!(step.sends, vec![EventSend::Started(event(1))]);
}

#[test]
fn the_host_starts_an_asked_event_by_its_data_id_and_tells_it() {
    let mut machine = host();
    let step = host_hears_ask(
        &mut machine,
        start_ask(event(1)),
        readings(JumpRole::Host, 1_000),
    );
    assert_eq!(step.calls, vec![EventCall::Load(event(1))]);
    let resent = host_hears_ask(
        &mut machine,
        start_ask(event(1)),
        readings(JumpRole::Host, 1_500),
    );
    assert!(resent.calls.is_empty());
    let played = machine.step(&EventReadings {
        slots: vec![loaded(event(1))],
        ..readings(JumpRole::Host, 1_600)
    });
    assert_eq!(played.calls, vec![EventCall::Play(event(1))]);
    machine.on_start(event(1), 1_700);
    let told = machine.step(&readings(JumpRole::Host, 1_710));
    assert_eq!(told.sends, vec![EventSend::Started(event(1))]);
    machine.on_end(event(1), false, 9_000);
    let ended = machine.step(&readings(JumpRole::Host, 9_010));
    assert!(ended.calls.contains(&EventCall::Release(event(1))));
    assert!(ended.sends.contains(&EventSend::Ended {
        id: event(1),
        skipped: false
    }));
}

#[test]
fn the_host_refuses_an_asked_start_it_cannot_make_and_says_why() {
    let cases = [
        (
            EventReadings {
                partner_place: StoryPlace::new(0x100, "B"),
                ..readings(JumpRole::Host, 1_000)
            },
            "another place",
        ),
        (
            EventReadings {
                event_flags_say_playing: true,
                ..readings(JumpRole::Host, 1_000)
            },
            "plays another event",
        ),
        (
            EventReadings {
                local_in_play: false,
                ..readings(JumpRole::Host, 1_000)
            },
            "not in play",
        ),
        (
            EventReadings {
                current_phase_id: 0xF00,
                ..readings(JumpRole::Host, 1_000)
            },
            "not a story phase",
        ),
    ];
    for (at, reason) in cases {
        let mut machine = host();
        let step = host_hears_ask(&mut machine, start_ask(event(1)), at);
        assert!(step.calls.is_empty(), "{reason}");
        assert!(
            step.logs.iter().any(|line| line.contains(reason)),
            "{reason}"
        );
    }
    let mut off = EventMachine::new(false);
    off.step(&readings(JumpRole::Host, 0));
    let step = host_hears_ask(
        &mut off,
        start_ask(event(1)),
        readings(JumpRole::Host, 1_000),
    );
    assert!(step.calls.is_empty());
    assert!(step.logs.iter().any(|line| line.contains("turned off")));
}

#[test]
fn an_asked_start_of_an_event_the_host_plays_tells_its_start_again() {
    let mut machine = host();
    machine.on_play_request(event(1), 10);
    machine.on_start(event(1), 20);
    machine.step(&readings(JumpRole::Host, 30));
    let step = host_hears_ask(
        &mut machine,
        start_ask(event(1)),
        readings(JumpRole::Host, 40),
    );
    assert_eq!(step.sends, vec![EventSend::Started(event(1))]);
    assert!(step.calls.is_empty());
}

#[test]
fn an_asked_start_of_an_event_the_host_just_ended_tells_its_start_and_end_again() {
    let mut machine = host();
    machine.on_play_request(event(1), 10);
    machine.on_start(event(1), 20);
    machine.on_end(event(1), true, 30);
    machine.step(&readings(JumpRole::Host, 40));
    let step = host_hears_ask(
        &mut machine,
        start_ask(event(1)),
        readings(JumpRole::Host, 50),
    );
    assert_eq!(
        step.sends,
        vec![
            EventSend::Started(event(1)),
            EventSend::Ended {
                id: event(1),
                skipped: true
            }
        ]
    );
}

#[test]
fn the_hosts_skip_ends_the_guests_event_at_once() {
    let mut machine = guest();
    guest_plays_with_the_host(&mut machine, event(1), 1_000);
    let step = guest_hears_end(&mut machine, event(1), true, 2_000);
    assert!(step.calls.contains(&EventCall::Skip));
}

#[test]
fn the_guests_event_may_outlast_the_hosts_normal_end_by_the_bound() {
    let mut machine = guest();
    guest_plays_with_the_host(&mut machine, event(1), 1_000);
    let step = guest_hears_end(&mut machine, event(1), false, 2_000);
    assert!(!step.calls.contains(&EventCall::Skip));
    let before = machine.step(&readings(
        JumpRole::Guest,
        2_000 + GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS - 1,
    ));
    assert!(!before.calls.contains(&EventCall::Skip));
    let due = machine.step(&readings(
        JumpRole::Guest,
        2_000 + GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS,
    ));
    assert!(due.calls.contains(&EventCall::Skip));
}

#[test]
fn a_guest_event_that_ends_on_its_own_after_the_hosts_end_is_not_skipped() {
    let mut machine = guest();
    guest_plays_with_the_host(&mut machine, event(1), 1_000);
    guest_hears_end(&mut machine, event(1), false, 2_000);
    machine.on_end(event(1), false, 3_000);
    let step = machine.step(&readings(JumpRole::Guest, 10_000));
    assert!(!step.calls.contains(&EventCall::Skip));
}

#[test]
fn a_guest_event_that_starts_after_the_hosts_skip_is_skipped_at_once() {
    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    guest_hears_end(&mut machine, event(1), true, 1_500);
    assert_eq!(machine.on_play_request(event(1), 2_000), PlayVerdict::Pass);
    machine.on_start(event(1), 2_010);
    let step = machine.step(&readings(JumpRole::Guest, 2_020));
    assert!(step.calls.contains(&EventCall::Skip));
}

#[test]
fn the_guests_skip_is_swallowed_and_asked_of_the_host() {
    let mut machine = guest();
    guest_plays_with_the_host(&mut machine, event(1), 1_000);
    assert_eq!(machine.on_player_skip(2_000), SkipVerdict::Swallow);
    let step = machine.step(&readings(JumpRole::Guest, 2_010));
    assert!(has_ask(&step, skip_ask(event(1))));
    assert!(!step.calls.contains(&EventCall::Skip));
    let skipped = guest_hears_end(&mut machine, event(1), true, 2_500);
    assert!(skipped.calls.contains(&EventCall::Skip));
}

#[test]
fn the_guests_skip_is_made_here_when_the_host_does_not_end_in_time() {
    let mut machine = guest();
    guest_plays_with_the_host(&mut machine, event(1), 1_000);
    machine.on_player_skip(2_000);
    let before = machine.step(&readings(
        JumpRole::Guest,
        2_000 + GUEST_SKIP_ASK_GIVE_UP_MS - 1,
    ));
    assert!(!before.calls.contains(&EventCall::Skip));
    let due = machine.step(&readings(
        JumpRole::Guest,
        2_000 + GUEST_SKIP_ASK_GIVE_UP_MS,
    ));
    assert!(due.calls.contains(&EventCall::Skip));
}

#[test]
fn the_host_skips_for_both_when_the_guest_asks_to_skip_what_it_plays() {
    let mut machine = host();
    machine.on_play_request(event(1), 10);
    machine.on_start(event(1), 20);
    machine.step(&readings(JumpRole::Host, 30));
    let step = host_hears_ask(
        &mut machine,
        skip_ask(event(1)),
        readings(JumpRole::Host, 40),
    );
    assert_eq!(step.calls, vec![EventCall::Skip]);
    let other = host_hears_ask(
        &mut machine,
        skip_ask(event(9)),
        readings(JumpRole::Host, 50),
    );
    assert!(other.calls.is_empty());
}

#[test]
fn the_hosts_own_skip_passes_and_a_guest_skip_of_an_unshared_event_passes() {
    let mut host = host();
    host.on_play_request(event(1), 10);
    host.on_start(event(1), 20);
    assert_eq!(host.on_player_skip(30), SkipVerdict::Pass);

    let mut machine = guest();
    guest_hears_start(&mut machine, event(1), 1_000);
    guest_hears_end(&mut machine, event(1), false, 1_200);
    machine.on_play_request(event(1), 2_000);
    machine.on_start(event(1), 2_010);
    machine.step(&readings(JumpRole::Guest, 2_020));
    assert_eq!(machine.on_player_skip(2_100), SkipVerdict::Pass);
}

#[test]
fn one_players_event_pause_never_pauses_or_holds_the_other() {
    let mut machine = guest();
    guest_plays_with_the_host(&mut machine, event(1), 1_000);
    for now_ms in (2_000..120_000).step_by(1_000) {
        let step = machine.step(&EventReadings {
            partner_playing: Some(event(1)),
            ..readings(JumpRole::Guest, now_ms)
        });
        assert!(step.calls.is_empty() && step.sends.is_empty());
    }
    let mut host = host();
    host.on_play_request(event(1), 10);
    host.on_start(event(1), 20);
    host.step(&readings(JumpRole::Host, 30));
    for now_ms in (1_000..120_000).step_by(1_000) {
        let step = host.step(&readings(JumpRole::Host, now_ms));
        assert!(step.calls.is_empty() && step.sends.is_empty());
    }
}

#[test]
fn with_the_link_down_everything_held_is_released_and_nothing_is_asked() {
    let mut machine = guest();
    machine.on_play_request(event(1), 100);
    machine.step(&readings(JumpRole::Guest, 100));
    let step = machine.step(&EventReadings {
        partner_present: false,
        ..readings(JumpRole::Guest, 500)
    });
    assert!(step.calls.contains(&EventCall::ReleaseHeldPlay(event(1))));
    assert!(step.sends.is_empty());
    assert_eq!(machine.on_play_request(event(2), 600), PlayVerdict::Pass);
}

#[test]
fn with_the_partner_out_of_play_a_swallowed_skip_is_made_here() {
    let mut machine = guest();
    guest_plays_with_the_host(&mut machine, event(1), 1_000);
    machine.on_player_skip(2_000);
    let step = machine.step(&EventReadings {
        partner_in_play: false,
        ..readings(JumpRole::Guest, 2_100)
    });
    assert!(step.calls.contains(&EventCall::Skip));
    assert!(step.sends.is_empty());
}

#[test]
fn solo_free_roam_and_menu_phases_never_hold_or_tell() {
    for at in [
        EventReadings {
            partner_present: false,
            ..readings(JumpRole::Guest, 100)
        },
        readings(JumpRole::Solo, 100),
        EventReadings {
            current_phase_id: 0xF10,
            ..readings(JumpRole::Guest, 100)
        },
    ] {
        let mut machine = EventMachine::new(true);
        machine.step(&at);
        assert_eq!(machine.on_play_request(event(1), 100), PlayVerdict::Pass);
        machine.on_start(event(1), 110);
        assert_eq!(machine.on_player_skip(120), SkipVerdict::Pass);
        machine.on_end(event(1), true, 130);
        let step = machine.step(&EventReadings { now_ms: 140, ..at });
        assert!(step.sends.is_empty());
        assert!(step.calls.is_empty());
    }
}

#[test]
fn a_start_for_the_partner_that_fails_to_load_is_released() {
    let mut failed = host();
    host_hears_ask(
        &mut failed,
        start_ask(event(1)),
        readings(JumpRole::Host, 1_000),
    );
    let step = failed.step(&EventReadings {
        slots: vec![EventSlot {
            state: EVENT_SLOT_LOAD_FAILED_STATE,
            ..loaded(event(1))
        }],
        ..readings(JumpRole::Host, 1_100)
    });
    assert_eq!(step.calls, vec![EventCall::Release(event(1))]);

    let mut slow = host();
    host_hears_ask(
        &mut slow,
        start_ask(event(1)),
        readings(JumpRole::Host, 1_000),
    );
    let step = slow.step(&readings(
        JumpRole::Host,
        1_000 + EVENT_STARTED_FOR_THE_PARTNER_LOAD_GIVE_UP_MS,
    ));
    assert_eq!(step.calls, vec![EventCall::Release(event(1))]);
}

#[test]
fn the_slot_table_is_read_into_live_slots() {
    let mut table = vec![0_u8; EVENT_SLOT_TABLE_BYTES];
    let second = EVENT_SLOT_STRIDE;
    table[second..second + 4].copy_from_slice(&1_u32.to_le_bytes());
    table[second + 4..second + 16].copy_from_slice(&event(0x42).to_bytes());
    table[second + 0x14..second + 0x18].copy_from_slice(&EVENT_SLOT_PLAYING_STATE.to_le_bytes());
    let dying = 2 * EVENT_SLOT_STRIDE;
    table[dying..dying + 4].copy_from_slice(&0x8000_0001_u32.to_le_bytes());
    let slots = slots_of_table(&table);
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].id, event(0x42));
    assert!(slots[0].is_playing());
}
