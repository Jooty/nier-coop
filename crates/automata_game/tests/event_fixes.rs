use automata_game::event_follow::{
    EVENT_FOLLOW_GRACE_MS, EVENT_PLAYED_JUST_BEFORE_A_PLACE_CHANGE_COUNTS_FOR_THE_NEW_PLACE_MS,
    EVENT_SLOT_LOADED_STATE, EventAsk, EventAskWhat, EventCall, EventId, EventMachine,
    EventReadings, EventSend, EventSlot, EventStep, GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS,
    PlayVerdict,
};
use automata_game::load_gate::StoryPlace;
use automata_game::travel::JumpRole;

fn normal(id: i32) -> EventId {
    EventId {
        kind: 0,
        id,
        sub: -1,
    }
}

fn phase_event() -> EventId {
    EventId {
        kind: 2,
        id: 0xE010,
        sub: 256,
    }
}

fn readings(role: JumpRole, now_ms: u64, place: &str) -> EventReadings {
    EventReadings {
        now_ms,
        role,
        partner_present: true,
        local_in_play: true,
        partner_in_play: true,
        current_phase_id: 0x100,
        current_place: StoryPlace::new(0x100, place),
        partner_place: StoryPlace::new(0x100, place),
        partner_playing: None,
        slots: Vec::new(),
        event_flags_say_playing: false,
        started_heard: Vec::new(),
        ended_heard: Vec::new(),
        asks: Vec::new(),
        partner_report: None,
        local_unattended: None,
        skip_asks_heard: Vec::new(),
        local_event_pause_open: false,
        pauses_heard: Vec::new(),
    }
}

fn host_at(machine: &mut EventMachine, now_ms: u64, place: &str) -> EventStep {
    machine.step(&readings(JumpRole::Host, now_ms, place))
}

fn guest_at(machine: &mut EventMachine, now_ms: u64) -> EventStep {
    machine.step(&readings(JumpRole::Guest, now_ms, "A"))
}

fn start_ask(id: EventId) -> EventAsk {
    EventAsk {
        id,
        what: EventAskWhat::Start,
    }
}

fn already_played(id: EventId) -> EventAsk {
    EventAsk {
        id,
        what: EventAskWhat::AlreadyPlayed,
    }
}

fn host_hears_ask(machine: &mut EventMachine, id: EventId, now_ms: u64, place: &str) -> EventStep {
    machine.step(&EventReadings {
        asks: vec![start_ask(id)],
        ..readings(JumpRole::Host, now_ms, place)
    })
}

fn new_machine(role: JumpRole) -> EventMachine {
    let mut machine = EventMachine::new(true);
    machine.step(&readings(role, 0, "A"));
    machine
}

fn loads(step: &EventStep, id: EventId) -> bool {
    step.calls.contains(&EventCall::Load(id))
}

#[test]
fn an_event_the_hosts_game_starts_by_itself_is_told() {
    // Case (a): 0x11 begins as 0x10 ends, not asked for by the script.
    let mut host = new_machine(JumpRole::Host);
    host.on_play_request(normal(0x10), 100);
    host.on_start(normal(0x10), 110);
    host_at(&mut host, 120, "A");
    host.on_end(normal(0x10), false, 18_000);
    host.on_start(normal(0x11), 18_000);
    let told = host_at(&mut host, 18_010, "A");
    assert!(told.sends.contains(&EventSend::Started(normal(0x11))));
    assert_eq!(told.playing_mirrored_event, Some(normal(0x11)));
    host.on_end(normal(0x11), false, 23_000);
    let ended = host_at(&mut host, 23_010, "A");
    assert!(ended.sends.contains(&EventSend::Ended {
        id: normal(0x11),
        skipped: false
    }));
}

#[test]
fn the_guest_follows_an_event_the_hosts_game_started_by_itself() {
    let mut guest = new_machine(JumpRole::Guest);
    guest.step(&EventReadings {
        started_heard: vec![normal(0x11)],
        ..readings(JumpRole::Guest, 18_000, "A")
    });
    let step = guest_at(&mut guest, 18_000 + EVENT_FOLLOW_GRACE_MS);
    assert!(loads(&step, normal(0x11)));
}

#[test]
fn a_replay_of_an_event_started_for_the_partner_is_still_not_told() {
    let mut host = new_machine(JumpRole::Host);
    host_hears_ask(&mut host, normal(1), 1_000, "A");
    host.step(&EventReadings {
        slots: vec![EventSlot {
            flags: 3,
            id: normal(1),
            state: EVENT_SLOT_LOADED_STATE,
        }],
        ..readings(JumpRole::Host, 1_100, "A")
    });
    host.on_start(normal(1), 1_200);
    host.on_end(normal(1), false, 5_000);
    host_at(&mut host, 5_010, "A");
    host.on_play_request(normal(1), 8_000);
    host.on_start(normal(1), 8_010);
    let step = host_at(&mut host, 8_020, "A");
    assert!(step.calls.contains(&EventCall::Skip));
    assert!(step.sends.is_empty());
}

#[test]
fn the_guests_own_play_that_just_ended_is_never_started_again_for_the_host() {
    // Case (b): the guest's script started it out of the rule; its end and the start for the
    // host fell in the same millisecond.
    let mut guest = new_machine(JumpRole::Guest);
    let out_of_rule = EventReadings {
        partner_in_play: false,
        ..readings(JumpRole::Guest, 21_878, "A")
    };
    guest.step(&out_of_rule);
    assert_eq!(
        guest.on_play_request(phase_event(), 21_878),
        PlayVerdict::Pass
    );
    guest.on_start(phase_event(), 21_878);
    guest.step(&EventReadings {
        started_heard: vec![phase_event()],
        now_ms: 21_895,
        ..out_of_rule.clone()
    });
    for now_ms in (22_000..47_000).step_by(1_000) {
        guest.step(&EventReadings {
            now_ms,
            ..out_of_rule.clone()
        });
    }
    guest.on_end(phase_event(), false, 47_885);
    let step = guest_at(&mut guest, 47_885);
    assert!(!loads(&step, phase_event()));
    let later = guest_at(&mut guest, 60_000);
    assert!(!loads(&later, phase_event()));
}

#[test]
fn the_guests_own_play_however_it_began_is_never_started_again_while_it_plays() {
    let mut guest = new_machine(JumpRole::Guest);
    guest.on_start(normal(0x11), 1_000);
    guest.step(&EventReadings {
        started_heard: vec![normal(0x11)],
        ..readings(JumpRole::Guest, 1_010, "A")
    });
    let step = guest_at(&mut guest, 1_010 + EVENT_FOLLOW_GRACE_MS);
    assert!(!loads(&step, normal(0x11)));
}

#[test]
fn an_ask_for_an_event_the_host_played_in_this_place_is_answered_already_played() {
    // Case (c): the host's game played 0x11 by itself; the guest asked 38 s later.
    let mut host = new_machine(JumpRole::Host);
    host.on_start(normal(0x11), 27_852);
    host_at(&mut host, 27_860, "A");
    host.on_end(normal(0x11), false, 27_957);
    host_at(&mut host, 27_960, "A");
    let step = host_hears_ask(&mut host, normal(0x11), 66_000, "A");
    assert_eq!(
        step.sends,
        vec![EventSend::Ask(already_played(normal(0x11)))]
    );
    assert!(!loads(&step, normal(0x11)));
    let much_later = host_hears_ask(&mut host, normal(0x11), 400_000, "A");
    assert_eq!(
        much_later.sends,
        vec![EventSend::Ask(already_played(normal(0x11)))]
    );
}

#[test]
fn the_guests_held_play_is_released_at_once_when_the_host_already_played_it() {
    let mut guest = new_machine(JumpRole::Guest);
    assert_eq!(
        guest.on_play_request(normal(0x11), 1_000),
        PlayVerdict::Hold
    );
    guest_at(&mut guest, 1_000);
    let step = guest.step(&EventReadings {
        asks: vec![already_played(normal(0x11))],
        ..readings(JumpRole::Guest, 1_300, "A")
    });
    assert!(
        step.calls
            .contains(&EventCall::ReleaseHeldPlay(normal(0x11)))
    );
    guest.on_start(normal(0x11), 1_320);
    for now_ms in (1_400..1_400 + 3 * GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS).step_by(500) {
        let step = guest_at(&mut guest, now_ms);
        assert!(step.calls.is_empty() && step.sends.is_empty());
    }
}

#[test]
fn an_event_played_just_before_a_place_change_counts_for_the_new_place() {
    let mut host = new_machine(JumpRole::Host);
    host_at(&mut host, 27_000, "A");
    host.on_start(normal(0x11), 27_852);
    host_at(&mut host, 27_856, "B");
    host.on_end(normal(0x11), false, 27_957);
    host_at(&mut host, 27_960, "B");
    let step = host_hears_ask(&mut host, normal(0x11), 66_000, "B");
    assert_eq!(
        step.sends,
        vec![EventSend::Ask(already_played(normal(0x11)))]
    );
    assert!(!loads(&step, normal(0x11)));
}

#[test]
fn what_was_played_in_an_earlier_place_is_forgotten_in_the_next() {
    let mut host = new_machine(JumpRole::Host);
    host.on_start(normal(0x11), 1_000);
    host.on_end(normal(0x11), false, 1_100);
    host_at(&mut host, 1_200, "A");
    host_at(
        &mut host,
        1_000 + EVENT_PLAYED_JUST_BEFORE_A_PLACE_CHANGE_COUNTS_FOR_THE_NEW_PLACE_MS + 1,
        "B",
    );
    let step = host_hears_ask(&mut host, normal(0x11), 10_000, "B");
    assert!(loads(&step, normal(0x11)));
}
