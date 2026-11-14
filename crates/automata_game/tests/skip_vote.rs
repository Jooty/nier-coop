use automata_game::event_follow::{
    EventAsk, EventAskWhat, EventCall, EventId, EventMachine, EventReadings, EventSend, EventStep,
    OWN_SKIP_ASK_LASTS_MS, PARTNER_REPORT_STALE_FOR_A_SKIP_VOTE_MS, PARTNER_SKIP_ASK_LIFETIME_MS,
    PartnerReport, SKIP_ASK_RESEND_MS, SkipPrompt, SkipVerdict, Unattended,
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

fn partner_playing(id: EventId, unattended: bool) -> Option<PartnerReport> {
    Some(PartnerReport {
        playing: Some(id),
        unattended,
        age_ms: 0,
    })
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
        partner_report: partner_playing(event(1), false),
        local_unattended: None,
        skip_asks_heard: Vec::new(),
        progress: None,
    }
}

fn with_ask(at: EventReadings, id: EventId) -> EventReadings {
    EventReadings {
        skip_asks_heard: vec![id],
        ..at
    }
}

fn host_in_event(vote: bool) -> EventMachine {
    let mut machine = EventMachine::new(true).with_skip_vote(vote);
    machine.step(&readings(JumpRole::Host, 0));
    machine.on_play_request(event(1), 10);
    machine.on_start(event(1), 20);
    machine.step(&readings(JumpRole::Host, 30));
    machine
}

fn guest_in_event(vote: bool) -> EventMachine {
    let mut machine = EventMachine::new(true).with_skip_vote(vote);
    machine.step(&EventReadings {
        started_heard: vec![event(1)],
        ..readings(JumpRole::Guest, 0)
    });
    machine.on_play_request(event(1), 10);
    machine.on_start(event(1), 20);
    machine.step(&readings(JumpRole::Guest, 30));
    machine
}

fn asks(step: &EventStep) -> bool {
    step.sends.contains(&EventSend::SkipAsk(event(1)))
}

#[test]
fn the_skip_runs_once_both_players_asked() {
    let mut host = host_in_event(true);
    assert_eq!(host.on_player_skip(1_000), SkipVerdict::Swallow);
    let host_step = host.step(&readings(JumpRole::Host, 1_010));
    assert!(asks(&host_step));
    assert_eq!(
        host_step.skip_prompt,
        Some(SkipPrompt::WaitingForThePartner)
    );
    assert!(!host_step.calls.contains(&EventCall::Skip));

    let mut guest = guest_in_event(true);
    let shown = guest.step(&with_ask(readings(JumpRole::Guest, 1_100), event(1)));
    assert_eq!(shown.skip_prompt, Some(SkipPrompt::PartnerWantsToSkip));
    assert!(shown.calls.is_empty() && !asks(&shown));
    assert_eq!(guest.on_player_skip(1_200), SkipVerdict::Swallow);
    let agreed = guest.step(&readings(JumpRole::Guest, 1_210));
    assert!(asks(&agreed));
    assert!(!agreed.calls.contains(&EventCall::Skip));
    assert!(agreed.logs.iter().any(|line| line.contains("agreed")));

    let ran = host.step(&with_ask(readings(JumpRole::Host, 1_300), event(1)));
    assert!(ran.calls.contains(&EventCall::Skip));
    assert_eq!(ran.skip_prompt, None);
}

#[test]
fn the_hosts_own_press_runs_the_skip_when_the_guest_asked_first() {
    let mut host = host_in_event(true);
    host.step(&with_ask(readings(JumpRole::Host, 1_000), event(1)));
    assert_eq!(host.on_player_skip(1_500), SkipVerdict::Pass);
}

#[test]
fn the_guests_vote_is_run_by_the_host_through_the_skipped_end() {
    let mut guest = guest_in_event(true);
    guest.step(&with_ask(readings(JumpRole::Guest, 1_000), event(1)));
    assert_eq!(guest.on_player_skip(1_100), SkipVerdict::Swallow);
    let step = guest.step(&EventReadings {
        ended_heard: vec![(event(1), true)],
        ..readings(JumpRole::Guest, 1_400)
    });
    assert!(step.calls.contains(&EventCall::Skip));
}

#[test]
fn a_partner_that_does_not_count_lets_the_askers_skip_run_at_once() {
    let cases = [
        (
            EventReadings {
                partner_report: partner_playing(event(2), false),
                ..readings(JumpRole::Host, 500)
            },
            "does not play this event",
        ),
        (
            EventReadings {
                partner_report: Some(PartnerReport {
                    playing: Some(event(1)),
                    unattended: false,
                    age_ms: PARTNER_REPORT_STALE_FOR_A_SKIP_VOTE_MS + 1,
                }),
                ..readings(JumpRole::Host, 500)
            },
            "ms old",
        ),
        (
            EventReadings {
                partner_report: None,
                ..readings(JumpRole::Host, 500)
            },
            "no state report",
        ),
    ];
    for (at, why) in cases {
        let mut host = host_in_event(true);
        host.step(&at);
        assert_eq!(host.on_player_skip(600), SkipVerdict::Pass, "{why}");
        let step = host.step(&readings(JumpRole::Host, 700));
        assert!(step.logs.iter().any(|line| line.contains(why)), "{why}");
    }
}

#[test]
fn with_the_link_down_the_askers_skip_runs_at_once() {
    let mut guest = guest_in_event(true);
    guest.step(&EventReadings {
        partner_present: false,
        ..readings(JumpRole::Guest, 500)
    });
    assert_eq!(guest.on_player_skip(600), SkipVerdict::Pass);
}

#[test]
fn when_the_host_does_not_play_the_event_the_guests_skip_runs_on_the_guest() {
    let mut guest = guest_in_event(true);
    guest.step(&EventReadings {
        partner_report: Some(PartnerReport {
            playing: None,
            unattended: false,
            age_ms: 0,
        }),
        ..readings(JumpRole::Guest, 500)
    });
    assert_eq!(guest.on_player_skip(600), SkipVerdict::Pass);
}

#[test]
fn a_held_ask_runs_here_when_the_partner_stops_counting() {
    let mut host = host_in_event(true);
    host.on_player_skip(1_000);
    host.step(&readings(JumpRole::Host, 1_010));
    let step = host.step(&EventReadings {
        partner_report: partner_playing(event(2), false),
        ..readings(JumpRole::Host, 1_500)
    });
    assert!(step.calls.contains(&EventCall::Skip));
}

#[test]
fn an_unattended_partner_counts_as_having_asked() {
    let unattended = EventReadings {
        partner_report: partner_playing(event(1), true),
        ..readings(JumpRole::Host, 500)
    };
    let mut host = host_in_event(true);
    host.step(&unattended);
    assert_eq!(host.on_player_skip(600), SkipVerdict::Pass);
    let step = host.step(&EventReadings {
        now_ms: 700,
        ..unattended.clone()
    });
    assert!(step.logs.iter().any(|line| line.contains("unattended")));

    let mut guest = guest_in_event(true);
    guest.step(&EventReadings {
        role: JumpRole::Guest,
        ..unattended.clone()
    });
    assert_eq!(guest.on_player_skip(600), SkipVerdict::Swallow);
    let step = guest.step(&EventReadings {
        role: JumpRole::Guest,
        now_ms: 700,
        ..unattended
    });
    assert!(asks(&step));
}

#[test]
fn an_unattended_copy_that_hears_the_partners_ask_agrees_by_itself() {
    for why in [
        Unattended::WindowUnfocused,
        Unattended::NotTheForegroundWindow,
        Unattended::PauseMenuOpen,
    ] {
        let mut host = host_in_event(true);
        let step = host.step(&EventReadings {
            local_unattended: Some(why),
            ..with_ask(readings(JumpRole::Host, 1_000), event(1))
        });
        assert!(step.calls.contains(&EventCall::Skip), "{why:?}");
        assert!(step.logs.iter().any(|line| line.contains(why.label())));

        let mut guest = guest_in_event(true);
        let step = guest.step(&EventReadings {
            local_unattended: Some(why),
            ..with_ask(readings(JumpRole::Guest, 1_000), event(1))
        });
        assert!(asks(&step), "{why:?}");
        assert!(!step.calls.contains(&EventCall::Skip));
    }
}

#[test]
fn each_way_of_being_unattended_is_found() {
    assert_eq!(Unattended::of(false, true, false), None);
    assert_eq!(
        Unattended::of(true, true, false),
        Some(Unattended::WindowUnfocused)
    );
    assert_eq!(
        Unattended::of(false, false, false),
        Some(Unattended::NotTheForegroundWindow)
    );
    assert_eq!(
        Unattended::of(false, true, true),
        Some(Unattended::PauseMenuOpen)
    );
}

#[test]
fn an_attended_copy_is_never_answered_for() {
    let mut host = host_in_event(true);
    for now_ms in (1_000..1_000 + OWN_SKIP_ASK_LASTS_MS).step_by(400) {
        let step = host.step(&with_ask(readings(JumpRole::Host, now_ms), event(1)));
        assert!(step.calls.is_empty() && !asks(&step));
        assert_eq!(step.skip_prompt, Some(SkipPrompt::PartnerWantsToSkip));
    }
    let mut guest = guest_in_event(true);
    guest.on_player_skip(1_000);
    let step = guest.step(&readings(JumpRole::Guest, 1_100));
    assert_eq!(step.skip_prompt, Some(SkipPrompt::WaitingForThePartner));
}

#[test]
fn the_ask_is_sent_again_every_resend_interval() {
    let mut host = host_in_event(true);
    host.on_player_skip(1_000);
    assert!(asks(&host.step(&readings(JumpRole::Host, 1_000))));
    assert!(!asks(&host.step(&readings(
        JumpRole::Host,
        1_000 + SKIP_ASK_RESEND_MS - 1
    ))));
    assert!(asks(
        &host.step(&readings(JumpRole::Host, 1_000 + SKIP_ASK_RESEND_MS))
    ));
}

#[test]
fn an_ask_that_is_not_agreed_lapses_and_the_event_goes_on() {
    let mut guest = guest_in_event(true);
    guest.on_player_skip(1_000);
    let before = guest.step(&readings(
        JumpRole::Guest,
        1_000 + OWN_SKIP_ASK_LASTS_MS - 1,
    ));
    assert_eq!(before.skip_prompt, Some(SkipPrompt::WaitingForThePartner));
    let lapsed = guest.step(&readings(JumpRole::Guest, 1_000 + OWN_SKIP_ASK_LASTS_MS));
    assert_eq!(lapsed.skip_prompt, None);
    assert!(lapsed.calls.is_empty() && !asks(&lapsed));
    assert!(lapsed.logs.iter().any(|line| line.contains("lapsed")));
    guest.step(&readings(JumpRole::Guest, 20_000));
    assert_eq!(guest.on_player_skip(20_000), SkipVerdict::Swallow);
}

#[test]
fn the_partners_ask_lapses_after_its_lifetime() {
    let mut host = host_in_event(true);
    host.step(&with_ask(readings(JumpRole::Host, 1_000), event(1)));
    let gone = host.step(&readings(
        JumpRole::Host,
        1_000 + PARTNER_SKIP_ASK_LIFETIME_MS,
    ));
    assert_eq!(gone.skip_prompt, None);
    assert_eq!(
        host.on_player_skip(1_000 + PARTNER_SKIP_ASK_LIFETIME_MS + 10),
        SkipVerdict::Swallow
    );
}

#[test]
fn a_new_event_or_the_events_end_clears_every_ask() {
    let mut host = host_in_event(true);
    host.on_player_skip(1_000);
    host.on_end(event(1), false, 1_200);
    let ended = host.step(&EventReadings {
        partner_report: partner_playing(event(2), false),
        ..readings(JumpRole::Host, 1_300)
    });
    assert_eq!(ended.skip_prompt, None);
    host.step(&with_ask(readings(JumpRole::Host, 1_400), event(1)));
    host.on_play_request(event(2), 1_500);
    host.on_start(event(2), 1_510);
    let step = host.step(&EventReadings {
        partner_report: partner_playing(event(2), false),
        ..readings(JumpRole::Host, 1_520)
    });
    assert_eq!(step.skip_prompt, None);
    assert!(!step.calls.contains(&EventCall::Skip));
}

#[test]
fn an_ask_heard_just_before_this_copys_event_starts_is_kept() {
    let mut host = EventMachine::new(true).with_skip_vote(true);
    host.step(&with_ask(readings(JumpRole::Host, 1_000), event(1)));
    host.on_play_request(event(1), 1_100);
    host.on_start(event(1), 1_200);
    let shown = host.step(&readings(JumpRole::Host, 1_300));
    assert_eq!(shown.skip_prompt, Some(SkipPrompt::PartnerWantsToSkip));
    assert_eq!(host.on_player_skip(1_400), SkipVerdict::Pass);
}

#[test]
fn a_guests_ask_is_never_made_locally_after_a_wait() {
    let mut guest = guest_in_event(true);
    guest.on_player_skip(1_000);
    for now_ms in (1_000..1_000 + OWN_SKIP_ASK_LASTS_MS + 2_000).step_by(250) {
        let step = guest.step(&readings(JumpRole::Guest, now_ms));
        assert!(!step.calls.contains(&EventCall::Skip));
        assert!(
            !step
                .sends
                .iter()
                .any(|send| matches!(send, EventSend::Ask(_)))
        );
    }
}

#[test]
fn with_the_vote_off_the_either_player_skip_of_c3_stays() {
    let mut guest = guest_in_event(false);
    assert_eq!(guest.on_player_skip(1_000), SkipVerdict::Swallow);
    let step = guest.step(&readings(JumpRole::Guest, 1_010));
    assert!(step.sends.contains(&EventSend::Ask(EventAsk {
        id: event(1),
        what: EventAskWhat::Skip,
    })));
    assert!(!asks(&step));
    assert_eq!(step.skip_prompt, None);
    let mut host = host_in_event(false);
    assert_eq!(host.on_player_skip(1_000), SkipVerdict::Pass);
}
