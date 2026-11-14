use automata_game::load_gate::StoryPlace;
use automata_game::story_follow::{
    AskVerdict, DEFAULT_FULL_REQUEST_PARAMS, FollowMachine, FollowReadings, FollowSend, FollowStep,
    GameRequestVerdict, HOST_KEEPS_AN_ASK_WHILE_BUSY_MS, RequestClass, RequestReadiness,
    STORY_ASK_GIVE_UP_MS, STORY_ASK_RESEND_MS, STORY_PLACE_CATCH_UP_AFTER_MS,
    STORY_WATCHING_KEPT_LOG_MS, StoryAsk, StoryAskAnswer, StoryMove, StoryRequest,
};
use automata_game::travel::JumpRole;

fn place(name: &str) -> StoryPlace {
    StoryPlace::new(0x100, name)
}

fn sub_phase(name: &str) -> StoryRequest {
    StoryRequest::SubPhase {
        id: 0x100,
        name: name.to_owned(),
    }
}

fn full(id: i32, name: &str) -> StoryRequest {
    StoryRequest::full_with_default_params(id, name)
}

fn readings(role: JumpRole, now_ms: u64) -> FollowReadings {
    FollowReadings {
        now_ms,
        role,
        partner_present: true,
        readiness: ready(),
        current_place: place("A"),
        host_place: place("A"),
        host_report_heard_at_ms: now_ms,
        host_in_play: true,
        host_waiting_at_load_end: false,
        this_copy_watches: false,
        partner_watches: false,
        moves: Vec::new(),
        asks: Vec::new(),
        answers: Vec::new(),
    }
}

fn host_at(machine: &mut FollowMachine, now_ms: u64) -> FollowStep {
    machine.step(&readings(JumpRole::Host, now_ms))
}

fn guest_at(machine: &mut FollowMachine, now_ms: u64) -> FollowStep {
    machine.step(&readings(JumpRole::Guest, now_ms))
}

fn a_move(number: u32, request: StoryRequest) -> StoryMove {
    StoryMove {
        number,
        from: place("A"),
        request,
    }
}

fn an_ask(number: u32, at: &str, request: StoryRequest) -> StoryAsk {
    StoryAsk {
        number,
        at: place(at),
        request,
    }
}

fn moves_sent(step: &FollowStep) -> Vec<StoryMove> {
    step.sends
        .iter()
        .filter_map(|send| match send {
            FollowSend::Move(story_move) => Some(story_move.clone()),
            _ => None,
        })
        .collect()
}

fn asks_sent(step: &FollowStep) -> Vec<StoryAsk> {
    step.sends
        .iter()
        .filter_map(|send| match send {
            FollowSend::Ask(ask) => Some(ask.clone()),
            _ => None,
        })
        .collect()
}

fn answers_sent(step: &FollowStep) -> Vec<StoryAskAnswer> {
    step.sends
        .iter()
        .filter_map(|send| match send {
            FollowSend::Answer(answer) => Some(*answer),
            _ => None,
        })
        .collect()
}

fn a_host() -> FollowMachine {
    let mut machine = FollowMachine::default();
    host_at(&mut machine, 0);
    machine
}

fn a_guest() -> FollowMachine {
    let mut machine = FollowMachine::default();
    guest_at(&mut machine, 0);
    machine
}

#[test]
fn a_target_from_0xf00_up_is_a_system_request() {
    for id in [0xF00, 0xF10, 0xF30, 0x1000] {
        assert_eq!(
            RequestClass::of(&full(id, "A"), &place("B")),
            RequestClass::System
        );
    }
}

#[test]
fn a_target_without_a_phase_is_a_system_request_too() {
    assert_eq!(
        RequestClass::of(&full(-1, "A"), &place("B")),
        RequestClass::System
    );
}

#[test]
fn a_target_equal_to_the_current_place_is_the_same_place() {
    assert_eq!(
        RequestClass::of(&sub_phase("A"), &place("A")),
        RequestClass::SamePlace
    );
    assert_eq!(
        RequestClass::of(&full(0x100, "A"), &place("A")),
        RequestClass::SamePlace
    );
}

#[test]
fn anything_else_is_a_story_request() {
    assert_eq!(
        RequestClass::of(&sub_phase("B"), &place("A")),
        RequestClass::Story
    );
    assert_eq!(
        RequestClass::of(&full(0x200, "A"), &place("A")),
        RequestClass::Story
    );
}

#[test]
fn a_full_request_without_a_name_is_a_story_request_even_in_the_current_phase() {
    assert_eq!(
        RequestClass::of(&full(0x100, ""), &place("A")),
        RequestClass::Story
    );
}

fn ready() -> RequestReadiness {
    RequestReadiness {
        scene_state: 1,
        phase_system_state: 2,
        requested_phase_id: -1,
        pending_phase_id: 0x100,
        current_phase_id: 0x100,
        pending_name: Some("A".to_owned()),
        current_name: Some("A".to_owned()),
    }
}

fn not_ready() -> RequestReadiness {
    RequestReadiness {
        scene_state: 0,
        ..ready()
    }
}

fn pending_unequal() -> RequestReadiness {
    RequestReadiness {
        pending_phase_id: 0x200,
        pending_name: Some("C".to_owned()),
        ..ready()
    }
}

fn guest_step_with(
    machine: &mut FollowMachine,
    now_ms: u64,
    readiness: RequestReadiness,
    moves: Vec<StoryMove>,
) -> FollowStep {
    let mut reading = readings(JumpRole::Guest, now_ms);
    reading.readiness = readiness;
    reading.moves = moves;
    machine.step(&reading)
}

fn waiting_lines(step: &FollowStep) -> Vec<String> {
    step.logs
        .iter()
        .filter(|line| line.starts_with("story: waiting"))
        .cloned()
        .collect()
}

#[test]
fn a_request_can_be_taken_when_all_four_readings_hold() {
    assert!(ready().can_take_a_request());
}

#[test]
fn a_request_cannot_be_taken_while_the_scene_is_not_running() {
    for scene_state in [0, 2, 3, 4] {
        assert!(
            !RequestReadiness {
                scene_state,
                ..ready()
            }
            .can_take_a_request()
        );
    }
}

#[test]
fn a_request_cannot_be_taken_while_the_phase_system_is_busy() {
    for phase_system_state in [0, 3, 5, 6, 8] {
        assert!(
            !RequestReadiness {
                phase_system_state,
                ..ready()
            }
            .can_take_a_request()
        );
    }
}

#[test]
fn a_request_cannot_be_taken_while_another_is_requested() {
    assert!(
        !RequestReadiness {
            requested_phase_id: 0x200,
            ..ready()
        }
        .can_take_a_request()
    );
}

#[test]
fn a_request_cannot_be_taken_while_pending_differs_from_current() {
    assert!(
        !RequestReadiness {
            pending_phase_id: 0x200,
            ..ready()
        }
        .can_take_a_request()
    );
    assert!(
        !RequestReadiness {
            pending_name: Some("B".to_owned()),
            ..ready()
        }
        .can_take_a_request()
    );
    assert!(
        !RequestReadiness {
            pending_name: None,
            current_name: None,
            ..ready()
        }
        .can_take_a_request()
    );
}

#[test]
fn system_and_same_place_requests_pass_on_both_sides_and_are_never_told_or_asked() {
    let mut host = a_host();
    let mut guest = a_guest();
    for machine in [&mut host, &mut guest] {
        assert_eq!(
            machine.on_game_request(&full(0xF00, "A"), &place("A"), 10),
            GameRequestVerdict::Pass
        );
        assert_eq!(
            machine.on_game_request(&sub_phase("A"), &place("A"), 10),
            GameRequestVerdict::Pass
        );
    }
    let host_step = host_at(&mut host, 20);
    let guest_step = guest_at(&mut guest, 20);
    assert!(host_step.sends.is_empty());
    assert!(guest_step.sends.is_empty());
}

#[test]
fn a_solo_game_passes_every_request_unseen() {
    let mut machine = FollowMachine::default();
    assert_eq!(
        machine.on_game_request(&sub_phase("B"), &place("A"), 10),
        GameRequestVerdict::Pass
    );
    let step = machine.step(&readings(JumpRole::Solo, 20));
    assert!(step.sends.is_empty());
    assert!(step.make.is_none());
}

#[test]
fn a_game_without_its_partner_in_the_session_acts_solo() {
    let mut machine = a_guest();
    let mut gone = readings(JumpRole::Guest, 10);
    gone.partner_present = false;
    machine.step(&gone);
    assert_eq!(machine.role(), JumpRole::Solo);
    assert_eq!(
        machine.on_game_request(&sub_phase("B"), &place("A"), 20),
        GameRequestVerdict::Pass
    );
}

#[test]
fn the_host_passes_a_story_request_and_tells_it_as_a_move_with_the_next_number() {
    let mut host = a_host();
    assert_eq!(
        host.on_game_request(&sub_phase("B"), &place("A"), 10),
        GameRequestVerdict::Pass
    );
    assert_eq!(
        host.on_game_request(&full(0x200, "C"), &place("A"), 11),
        GameRequestVerdict::Pass
    );
    let step = host_at(&mut host, 20);
    let moves = moves_sent(&step);
    assert_eq!(moves.len(), 2);
    assert_eq!((moves[0].number, moves[1].number), (1, 2));
    assert_eq!(moves[0].request, sub_phase("B"));
    assert_eq!(moves[0].from, place("A"));
    assert_eq!(moves[1].request, full(0x200, "C"));
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("told the partner move 1"))
    );
}

#[test]
fn a_move_is_told_once() {
    let mut host = a_host();
    host.on_game_request(&sub_phase("B"), &place("A"), 10);
    assert_eq!(moves_sent(&host_at(&mut host, 20)).len(), 1);
    assert!(moves_sent(&host_at(&mut host, 30)).is_empty());
}

#[test]
fn an_ask_is_made_in_the_hosts_game_when_the_host_is_at_the_asks_place_and_can_take_it() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(7, "A", sub_phase("B")));
    let step = host.step(&at);
    assert_eq!(step.make, Some(sub_phase("B")));
    assert_eq!(
        answers_sent(&step),
        vec![StoryAskAnswer {
            number: 7,
            verdict: AskVerdict::Made
        }]
    );
    let moves = moves_sent(&step);
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].request, sub_phase("B"));
    assert_eq!(moves[0].from, place("A"));
}

#[test]
fn an_ask_the_host_made_is_told_with_the_next_move_number() {
    let mut host = a_host();
    host.on_game_request(&sub_phase("B"), &place("A"), 10);
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(7, "A", sub_phase("C")));
    let step = host.step(&at);
    let numbers: Vec<u32> = moves_sent(&step).iter().map(|m| m.number).collect();
    assert_eq!(numbers, vec![1, 2]);
}

#[test]
fn an_ask_whose_target_is_the_hosts_last_move_is_already_made() {
    let mut host = a_host();
    host.on_game_request(&sub_phase("B"), &place("A"), 10);
    host_at(&mut host, 20);
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(7, "A", sub_phase("B")));
    let step = host.step(&at);
    assert!(step.make.is_none());
    assert_eq!(answers_sent(&step)[0].verdict, AskVerdict::AlreadyMade);
    assert!(moves_sent(&step).is_empty());
}

#[test]
fn an_ask_whose_target_is_where_the_host_already_is_is_already_made() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.current_place = place("B");
    at.asks.push(an_ask(7, "B", full(0x100, "B")));
    let step = host.step(&at);
    assert_eq!(answers_sent(&step)[0].verdict, AskVerdict::AlreadyMade);
}

#[test]
fn an_ask_from_another_place_is_refused() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(7, "Z", sub_phase("B")));
    let step = host.step(&at);
    assert!(step.make.is_none());
    assert_eq!(
        answers_sent(&step)[0].verdict,
        AskVerdict::RefusedOtherPlace
    );
}

#[test]
fn an_ask_with_no_place_known_is_refused_as_another_place() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(7, "", sub_phase("B")));
    let step = host.step(&at);
    assert_eq!(
        answers_sent(&step)[0].verdict,
        AskVerdict::RefusedOtherPlace
    );
}

#[test]
fn an_ask_is_kept_while_the_host_cannot_take_a_request_and_made_when_it_can() {
    let mut host = a_host();
    let mut busy = readings(JumpRole::Host, 100);
    busy.readiness = not_ready();
    busy.asks.push(an_ask(7, "A", sub_phase("B")));
    let step = host.step(&busy);
    assert!(step.make.is_none());
    assert!(answers_sent(&step).is_empty());
    let mut still_busy = readings(JumpRole::Host, 100 + HOST_KEEPS_AN_ASK_WHILE_BUSY_MS - 1);
    still_busy.readiness = not_ready();
    assert!(answers_sent(&host.step(&still_busy)).is_empty());
    let free = readings(JumpRole::Host, 100 + HOST_KEEPS_AN_ASK_WHILE_BUSY_MS - 1);
    let step = host.step(&free);
    assert_eq!(step.make, Some(sub_phase("B")));
    assert_eq!(answers_sent(&step)[0].verdict, AskVerdict::Made);
}

#[test]
fn an_ask_kept_ten_seconds_while_busy_is_refused_as_busy_too_long() {
    let mut host = a_host();
    let mut busy = readings(JumpRole::Host, 100);
    busy.readiness = not_ready();
    busy.asks.push(an_ask(7, "A", sub_phase("B")));
    host.step(&busy);
    let mut later = readings(JumpRole::Host, 100 + HOST_KEEPS_AN_ASK_WHILE_BUSY_MS);
    later.readiness = not_ready();
    let step = host.step(&later);
    assert_eq!(
        answers_sent(&step),
        vec![StoryAskAnswer {
            number: 7,
            verdict: AskVerdict::RefusedBusyTooLong
        }]
    );
    assert!(step.make.is_none());
}

#[test]
fn a_resent_ask_gets_its_answer_again_and_is_not_made_twice() {
    let mut host = a_host();
    let mut first = readings(JumpRole::Host, 100);
    first.asks.push(an_ask(7, "A", sub_phase("B")));
    host.step(&first);
    let mut again = readings(JumpRole::Host, 1_100);
    again.current_place = place("A");
    again.asks.push(an_ask(7, "A", sub_phase("B")));
    let step = host.step(&again);
    assert!(step.make.is_none());
    assert_eq!(answers_sent(&step)[0].verdict, AskVerdict::Made);
    assert!(moves_sent(&step).is_empty());
}

#[test]
fn only_one_request_is_made_in_one_step() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(7, "A", sub_phase("B")));
    at.asks.push(an_ask(8, "A", sub_phase("C")));
    let step = host.step(&at);
    assert_eq!(step.make, Some(sub_phase("B")));
    assert_eq!(answers_sent(&step).len(), 1);
    let mut busy = readings(JumpRole::Host, 200);
    busy.readiness = not_ready();
    assert!(host.step(&busy).make.is_none());
    let step = host.step(&readings(JumpRole::Host, 300));
    assert_eq!(step.make, Some(sub_phase("C")));
}

#[test]
fn a_sub_phase_ask_for_another_phase_id_is_made_in_the_hosts_game_as_a_full_request() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(
        7,
        "A",
        StoryRequest::SubPhase {
            id: 0x200,
            name: "B".to_owned(),
        },
    ));
    let step = host.step(&at);
    assert_eq!(
        step.make,
        Some(StoryRequest::Full {
            id: 0x200,
            name: "B".to_owned(),
            params: DEFAULT_FULL_REQUEST_PARAMS
        })
    );
}

#[test]
fn the_guest_swallows_a_story_request_and_asks_the_host_at_once() {
    let mut guest = a_guest();
    assert_eq!(
        guest.on_game_request(&sub_phase("B"), &place("A"), 10),
        GameRequestVerdict::Swallow
    );
    let step = guest_at(&mut guest, 20);
    let asks = asks_sent(&step);
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].request, sub_phase("B"));
    assert_eq!(asks[0].at, place("A"));
    assert!(step.make.is_none());
}

#[test]
fn the_guest_asks_again_every_second_until_something_answers() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 10);
    let first = guest_at(&mut guest, 20);
    let number = asks_sent(&first)[0].number;
    assert!(asks_sent(&guest_at(&mut guest, 20 + STORY_ASK_RESEND_MS - 1)).is_empty());
    let again = guest_at(&mut guest, 20 + STORY_ASK_RESEND_MS);
    assert_eq!(asks_sent(&again)[0].number, number);
    assert!(
        again
            .logs
            .iter()
            .any(|line| line.contains("kept for") && line.contains("still no word"))
    );
    assert!(asks_sent(&guest_at(&mut guest, 20 + STORY_ASK_RESEND_MS + 10)).is_empty());
    assert_eq!(
        asks_sent(&guest_at(&mut guest, 20 + 2 * STORY_ASK_RESEND_MS)).len(),
        1
    );
}

#[test]
fn a_newer_request_replaces_the_kept_one() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 10);
    guest.on_game_request(&sub_phase("C"), &place("A"), 11);
    let step = guest_at(&mut guest, 20);
    let asks = asks_sent(&step);
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].request, sub_phase("C"));
    assert!(step.logs.iter().any(|line| line.contains("is replaced by")));
}

#[test]
fn a_move_from_the_host_is_made_here_when_this_game_can_take_a_request() {
    let mut guest = a_guest();
    let mut at = readings(JumpRole::Guest, 100);
    at.moves.push(a_move(1, sub_phase("B")));
    let step = guest.step(&at);
    assert_eq!(step.make, Some(sub_phase("B")));
}

#[test]
fn a_move_waits_in_the_queue_while_this_game_cannot_take_a_request() {
    let mut guest = a_guest();
    let mut busy = readings(JumpRole::Guest, 100);
    busy.readiness = not_ready();
    busy.moves.push(a_move(1, sub_phase("B")));
    assert!(guest.step(&busy).make.is_none());
    let step = guest.step(&readings(JumpRole::Guest, 450));
    assert_eq!(step.make, Some(sub_phase("B")));
    assert!(step.logs.iter().any(|line| line.contains("after 350 ms")));
}

#[test]
fn moves_are_made_in_order_one_per_step() {
    let mut guest = a_guest();
    let mut at = readings(JumpRole::Guest, 100);
    at.moves.push(a_move(2, sub_phase("C")));
    at.moves.push(a_move(1, sub_phase("B")));
    assert_eq!(guest.step(&at).make, Some(sub_phase("B")));
    let mut next = readings(JumpRole::Guest, 200);
    next.current_place = place("B");
    assert_eq!(guest.step(&next).make, Some(sub_phase("C")));
}

#[test]
fn a_repeated_move_number_is_ignored() {
    let mut guest = a_guest();
    let mut at = readings(JumpRole::Guest, 100);
    at.moves.push(a_move(1, sub_phase("B")));
    guest.step(&at);
    let mut again = readings(JumpRole::Guest, 200);
    again.current_place = place("A");
    again.moves.push(a_move(1, sub_phase("B")));
    assert!(guest.step(&again).make.is_none());
}

#[test]
fn a_move_whose_target_is_this_games_place_is_dropped_as_already_here() {
    let mut guest = a_guest();
    let mut at = readings(JumpRole::Guest, 100);
    at.moves.push(a_move(1, sub_phase("A")));
    let step = guest.step(&at);
    assert!(step.make.is_none());
    assert!(step.logs.iter().any(|line| line.contains("already here")));
}

#[test]
fn a_move_for_the_request_already_in_flight_is_dropped_as_already_here() {
    let mut guest = a_guest();
    let mut at = readings(JumpRole::Guest, 100);
    at.moves.push(a_move(1, sub_phase("B")));
    assert!(guest.step(&at).make.is_some());
    let mut under_way = readings(JumpRole::Guest, 200);
    under_way.readiness = ready();
    under_way.moves.push(a_move(2, sub_phase("B")));
    let step = guest.step(&under_way);
    assert!(step.make.is_none());
    assert!(step.logs.iter().any(|line| line.contains("already here")));
}

#[test]
fn a_sub_phase_move_for_another_phase_id_is_made_as_a_full_request_with_default_params() {
    let mut guest = a_guest();
    let mut at = readings(JumpRole::Guest, 100);
    at.moves.push(a_move(
        1,
        StoryRequest::SubPhase {
            id: 0x200,
            name: "B".to_owned(),
        },
    ));
    assert_eq!(
        guest.step(&at).make,
        Some(StoryRequest::Full {
            id: 0x200,
            name: "B".to_owned(),
            params: [0, 0, 0x78, 0, 0, 0]
        })
    );
}

#[test]
fn a_full_move_is_made_as_it_was_told() {
    let mut guest = a_guest();
    let request = StoryRequest::Full {
        id: 0x200,
        name: "B".to_owned(),
        params: [2, 1, 30, 1, 0, 9],
    };
    let mut at = readings(JumpRole::Guest, 100);
    at.moves.push(a_move(1, request.clone()));
    assert_eq!(guest.step(&at).make, Some(request));
}

#[test]
fn a_move_clears_the_kept_request() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 10);
    guest_at(&mut guest, 20);
    let mut busy = readings(JumpRole::Guest, 30);
    busy.readiness = not_ready();
    busy.moves.push(a_move(1, sub_phase("C")));
    guest.step(&busy);
    assert!(asks_sent(&guest_at(&mut guest, 5_000)).is_empty());
}

#[test]
fn an_already_made_answer_clears_the_kept_request() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 10);
    let number = asks_sent(&guest_at(&mut guest, 20))[0].number;
    let mut at = readings(JumpRole::Guest, 30);
    at.answers.push(StoryAskAnswer {
        number,
        verdict: AskVerdict::AlreadyMade,
    });
    guest.step(&at);
    assert!(asks_sent(&guest_at(&mut guest, 5_000)).is_empty());
    assert!(guest_at(&mut guest, 40_000).make.is_none());
}

#[test]
fn a_refusal_drops_the_kept_request_with_its_reason_and_it_does_not_run_here() {
    for verdict in [
        AskVerdict::RefusedOtherPlace,
        AskVerdict::RefusedBusyTooLong,
    ] {
        let mut guest = a_guest();
        guest.on_game_request(&sub_phase("B"), &place("A"), 10);
        let number = asks_sent(&guest_at(&mut guest, 20))[0].number;
        let mut at = readings(JumpRole::Guest, 30);
        at.answers.push(StoryAskAnswer { number, verdict });
        let step = guest.step(&at);
        assert!(step.logs.iter().any(|line| line.contains(verdict.label())));
        assert!(step.make.is_none());
        assert!(asks_sent(&guest_at(&mut guest, 5_000)).is_empty());
        assert!(guest_at(&mut guest, 40_000).make.is_none());
    }
}

#[test]
fn an_answer_for_another_ask_changes_nothing() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 10);
    let number = asks_sent(&guest_at(&mut guest, 20))[0].number;
    let mut at = readings(JumpRole::Guest, 30);
    at.answers.push(StoryAskAnswer {
        number: number + 5,
        verdict: AskVerdict::RefusedOtherPlace,
    });
    guest.step(&at);
    assert_eq!(
        asks_sent(&guest_at(&mut guest, 20 + STORY_ASK_RESEND_MS)).len(),
        1
    );
}

#[test]
fn after_twenty_seconds_without_move_or_answer_the_kept_request_runs_here_loudly() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 100);
    guest_at(&mut guest, 100);
    let before = guest_at(&mut guest, 100 + STORY_ASK_GIVE_UP_MS - 1);
    assert!(before.make.is_none());
    let step = guest_at(&mut guest, 100 + STORY_ASK_GIVE_UP_MS);
    assert_eq!(step.make, Some(sub_phase("B")));
    assert!(step.logs.iter().any(|line| line.contains("runs here")));
}

#[test]
fn a_given_up_request_waits_for_the_game_to_take_requests() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 100);
    guest_at(&mut guest, 100);
    let mut busy = readings(JumpRole::Guest, 100 + STORY_ASK_GIVE_UP_MS);
    busy.readiness = not_ready();
    assert!(guest.step(&busy).make.is_none());
    let step = guest_at(&mut guest, 100 + STORY_ASK_GIVE_UP_MS + 500);
    assert_eq!(step.make, Some(sub_phase("B")));
}

#[test]
fn when_the_partner_is_gone_the_kept_request_runs_here_at_once_and_the_queue_is_cleared() {
    let mut guest = a_guest();
    let mut busy = readings(JumpRole::Guest, 50);
    busy.readiness = not_ready();
    busy.moves.push(a_move(1, sub_phase("C")));
    guest.step(&busy);
    guest.on_game_request(&sub_phase("B"), &place("A"), 60);
    let mut gone = readings(JumpRole::Guest, 100);
    gone.partner_present = false;
    let step = guest.step(&gone);
    assert_eq!(step.make, Some(sub_phase("B")));
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("partner is gone"))
    );
    let mut after = readings(JumpRole::Solo, 200);
    after.partner_present = false;
    assert!(guest.step(&after).make.is_none());
}

#[test]
fn a_released_request_waits_for_the_game_when_the_link_is_gone() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 60);
    let mut gone = readings(JumpRole::Guest, 100);
    gone.partner_present = false;
    gone.readiness = not_ready();
    assert!(guest.step(&gone).make.is_none());
    gone.now_ms = 150;
    gone.readiness = ready();
    assert_eq!(guest.step(&gone).make, Some(sub_phase("B")));
}

fn differing(now_ms: u64, host: &str, here: &str) -> FollowReadings {
    let mut at = readings(JumpRole::Guest, now_ms);
    at.host_place = place(host);
    at.current_place = place(here);
    at
}

#[test]
fn the_guest_catches_up_after_five_seconds_of_difference_with_a_sub_phase_in_the_same_phase() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "B", "A"));
    let early = guest.step(&differing(
        1_000 + STORY_PLACE_CATCH_UP_AFTER_MS - 1,
        "B",
        "A",
    ));
    assert!(early.make.is_none());
    let step = guest.step(&differing(1_000 + STORY_PLACE_CATCH_UP_AFTER_MS, "B", "A"));
    assert_eq!(step.make, Some(sub_phase("B")));
    assert!(step.logs.iter().any(|line| line.contains("catching up")));
}

#[test]
fn the_guest_catches_up_with_a_full_request_when_the_phase_id_differs() {
    let mut guest = a_guest();
    let mut start = differing(1_000, "B", "A");
    start.host_place = StoryPlace::new(0x200, "B");
    guest.step(&start);
    let mut late = differing(1_000 + STORY_PLACE_CATCH_UP_AFTER_MS, "B", "A");
    late.host_place = StoryPlace::new(0x200, "B");
    assert_eq!(guest.step(&late).make, Some(full(0x200, "B")));
}

#[test]
fn a_break_in_the_difference_restarts_the_five_seconds() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "B", "A"));
    guest.step(&differing(4_000, "A", "A"));
    guest.step(&differing(4_500, "B", "A"));
    let step = guest.step(&differing(6_000, "B", "A"));
    assert!(step.make.is_none());
    let step = guest.step(&differing(4_500 + STORY_PLACE_CATCH_UP_AFTER_MS, "B", "A"));
    assert_eq!(step.make, Some(sub_phase("B")));
}

#[test]
fn no_catch_up_while_the_hosts_report_is_stale() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "B", "A"));
    let mut late = differing(7_000, "B", "A");
    late.host_report_heard_at_ms = 3_000;
    assert!(guest.step(&late).make.is_none());
    let mut never = differing(7_100, "B", "A");
    never.host_report_heard_at_ms = 0;
    assert!(guest.step(&never).make.is_none());
}

#[test]
fn no_catch_up_unless_the_host_is_in_play_or_waiting_at_a_loads_end() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "B", "A"));
    let mut away = differing(7_000, "B", "A");
    away.host_in_play = false;
    away.host_waiting_at_load_end = false;
    assert!(guest.step(&away).make.is_none());
    let mut waiting = differing(7_100, "B", "A");
    waiting.host_in_play = false;
    waiting.host_waiting_at_load_end = true;
    assert_eq!(guest.step(&waiting).make, Some(sub_phase("B")));
}

#[test]
fn a_queued_move_is_made_before_any_catch_up() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "B", "A"));
    let mut late = differing(7_000, "B", "A");
    late.moves.push(a_move(1, sub_phase("C")));
    assert_eq!(guest.step(&late).make, Some(sub_phase("C")));
}

#[test]
fn no_catch_up_while_a_request_is_in_flight() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "B", "A"));
    let mut first = differing(2_000, "B", "A");
    first.moves.push(a_move(1, sub_phase("B")));
    assert!(guest.step(&first).make.is_some());
    let step = guest.step(&differing(9_000, "B", "A"));
    assert!(step.make.is_none());
}

#[test]
fn no_catch_up_while_this_game_cannot_take_a_request() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "B", "A"));
    let mut busy = differing(7_000, "B", "A");
    busy.readiness = not_ready();
    assert!(guest.step(&busy).make.is_none());
}

#[test]
fn no_catch_up_to_or_from_a_system_phase() {
    let mut guest = a_guest();
    let mut start = differing(1_000, "B", "A");
    start.host_place = StoryPlace::new(0xF00, "B");
    guest.step(&start);
    let mut late = differing(7_000, "B", "A");
    late.host_place = StoryPlace::new(0xF00, "B");
    assert!(guest.step(&late).make.is_none());
}

#[test]
fn no_catch_up_when_a_place_has_no_name() {
    let mut guest = a_guest();
    guest.step(&differing(1_000, "", "A"));
    assert!(guest.step(&differing(7_000, "", "A")).make.is_none());
}

#[test]
fn a_request_in_flight_is_forgotten_when_this_game_arrives_or_after_fifteen_seconds() {
    let mut guest = a_guest();
    let mut first = readings(JumpRole::Guest, 100);
    first.moves.push(a_move(1, sub_phase("B")));
    assert!(guest.step(&first).make.is_some());
    let mut arrived = readings(JumpRole::Guest, 200);
    arrived.current_place = place("B");
    guest.step(&arrived);
    let mut back = readings(JumpRole::Guest, 300);
    back.current_place = place("B");
    back.moves.push(a_move(2, sub_phase("A")));
    assert_eq!(guest.step(&back).make, Some(sub_phase("A")));
}

#[test]
fn a_request_that_never_arrives_stops_counting_as_under_way() {
    let mut guest = a_guest();
    let mut first = readings(JumpRole::Guest, 100);
    first.moves.push(a_move(1, sub_phase("B")));
    assert!(guest.step(&first).make.is_some());
    let mut late = readings(JumpRole::Guest, 100 + 15_000);
    late.moves.push(a_move(2, sub_phase("B")));
    let step = guest.step(&late);
    assert_eq!(step.make, Some(sub_phase("B")));
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("no longer counted"))
    );
}

#[test]
fn the_log_lines_name_the_numbers_the_places_and_the_reasons() {
    use automata_game::story_follow::{
        answer_heard_line, ask_sent_line, catch_up_line, host_ask_answered_line, link_lost_line,
        made_here_line, move_heard_line, move_told_line,
    };
    let story_move = a_move(5, sub_phase("B"));
    assert!(move_told_line(&story_move).contains("move 5"));
    assert!(move_heard_line(&story_move, 2).contains("2 queued"));
    let ask = an_ask(3, "A", sub_phase("B"));
    assert!(ask_sent_line(&ask).contains("ask 3"));
    let answer = StoryAskAnswer {
        number: 3,
        verdict: AskVerdict::RefusedOtherPlace,
    };
    assert!(answer_heard_line(&answer, &sub_phase("B")).contains("another place"));
    assert!(host_ask_answered_line(3, AskVerdict::RefusedBusyTooLong).contains("busy too long"));
    assert!(made_here_line("why", &sub_phase("B"), 120).contains("120 ms"));
    assert!(catch_up_line(&place("B"), &place("A"), 5_200, &sub_phase("B")).contains("5200 ms"));
    assert!(link_lost_line(true, 2).contains("runs here"));
}

#[test]
fn a_request_the_wire_would_refuse_passes_unseen_on_both_sides() {
    let unusual = StoryRequest::SubPhase {
        id: 0x100,
        name: "N".repeat(32),
    };
    for mut machine in [a_host(), a_guest()] {
        assert_eq!(
            machine.on_game_request(&unusual, &place("A"), 10),
            GameRequestVerdict::Pass
        );
        let step = machine.step(&readings(machine.role(), 20));
        assert!(step.sends.is_empty());
        assert!(step.logs.iter().any(|line| line.contains("passed unseen")));
    }
}

#[test]
fn a_wanted_request_that_cannot_be_made_says_why_once_a_second() {
    let mut guest = a_guest();
    let heard = vec![a_move(1, sub_phase("B"))];
    let mut lines = Vec::new();
    for now_ms in [1_000, 1_500, 1_999, 2_000, 2_500, 3_000] {
        let moves = if now_ms == 1_000 {
            heard.clone()
        } else {
            Vec::new()
        };
        let step = guest_step_with(&mut guest, now_ms, not_ready(), moves);
        assert!(step.make.is_none());
        lines.extend(waiting_lines(&step));
    }
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        "story: waiting 1000 ms to make sub-phase 0x100 B: this game cannot take a request (scene state 0, phase state 2, requested id -1, pending 0x100 A, current 0x100 A)"
    );
    assert!(lines[1].starts_with("story: waiting 2000 ms to make sub-phase 0x100 B"));
}

#[test]
fn the_waiting_line_names_the_readings_that_block() {
    let mut guest = a_guest();
    let blocked = RequestReadiness {
        phase_system_state: 5,
        requested_phase_id: 0x200,
        ..ready()
    };
    guest_step_with(
        &mut guest,
        100,
        blocked.clone(),
        vec![a_move(1, sub_phase("B"))],
    );
    let step = guest_step_with(&mut guest, 1_100, blocked, Vec::new());
    let lines = waiting_lines(&step);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains(
        "scene state 1, phase state 5, requested id 512, pending 0x100 A, current 0x100 A"
    ));
}

#[test]
fn no_waiting_line_when_nothing_is_wanted() {
    let mut guest = a_guest();
    let mut host = a_host();
    for now_ms in (0..=5_000).step_by(250) {
        let step = guest_step_with(&mut guest, now_ms, not_ready(), Vec::new());
        assert!(waiting_lines(&step).is_empty());
        let mut reading = readings(JumpRole::Host, now_ms);
        reading.readiness = not_ready();
        assert!(waiting_lines(&host.step(&reading)).is_empty());
    }
}

#[test]
fn a_host_with_an_ask_it_is_about_to_make_says_why_it_waits() {
    let mut host = a_host();
    let mut reading = readings(JumpRole::Host, 100);
    reading.readiness = not_ready();
    reading.asks = vec![an_ask(1, "A", sub_phase("B"))];
    host.step(&reading);
    let mut later = readings(JumpRole::Host, 1_100);
    later.readiness = not_ready();
    let lines = waiting_lines(&host.step(&later));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("story: waiting 1000 ms to make sub-phase 0x100 B"));
}

#[test]
fn an_unequal_pending_phase_is_tolerated_only_after_3000_ms_without_a_break() {
    let mut guest = a_guest();
    let heard = vec![a_move(1, sub_phase("B"))];
    let step = guest_step_with(&mut guest, 1_000, pending_unequal(), heard);
    assert!(step.make.is_none());
    let step = guest_step_with(&mut guest, 3_999, pending_unequal(), Vec::new());
    assert!(step.make.is_none());
    assert!(
        !step
            .logs
            .iter()
            .any(|line| line.contains("requests are made anyway"))
    );
    let step = guest_step_with(&mut guest, 4_000, pending_unequal(), Vec::new());
    assert_eq!(step.make, Some(sub_phase("B")));
    let tolerated: Vec<_> = step
        .logs
        .iter()
        .filter(|line| line.contains("requests are made anyway"))
        .collect();
    assert_eq!(
        tolerated,
        [
            "story: pending 0x200 C has differed from current 0x100 A for 3000 ms with the phase system idle, requests are made anyway"
        ]
    );
}

#[test]
fn a_break_resets_the_tolerance_and_it_is_logged_once_per_stretch() {
    let mut guest = a_guest();
    let heard = vec![a_move(1, sub_phase("B")), a_move(2, sub_phase("D"))];
    guest_step_with(&mut guest, 1_000, pending_unequal(), heard);
    let broken = RequestReadiness {
        phase_system_state: 0,
        ..pending_unequal()
    };
    guest_step_with(&mut guest, 2_000, broken, Vec::new());
    guest_step_with(&mut guest, 3_000, pending_unequal(), Vec::new());
    let step = guest_step_with(&mut guest, 5_999, pending_unequal(), Vec::new());
    assert!(step.make.is_none());
    let step = guest_step_with(&mut guest, 6_000, pending_unequal(), Vec::new());
    assert_eq!(step.make, Some(sub_phase("B")));
    let step = guest_step_with(&mut guest, 6_100, pending_unequal(), Vec::new());
    assert_eq!(step.make, Some(sub_phase("D")));
    assert!(
        !step
            .logs
            .iter()
            .any(|line| line.contains("requests are made anyway"))
    );
}

#[test]
fn the_tolerance_never_applies_unless_the_scene_runs_the_phase_system_is_idle_and_nothing_is_requested()
 {
    let blockers = [
        RequestReadiness {
            scene_state: 2,
            ..pending_unequal()
        },
        RequestReadiness {
            phase_system_state: 5,
            ..pending_unequal()
        },
        RequestReadiness {
            requested_phase_id: 0x200,
            ..pending_unequal()
        },
    ];
    for blocked in blockers {
        let mut guest = a_guest();
        guest_step_with(
            &mut guest,
            1_000,
            blocked.clone(),
            vec![a_move(1, sub_phase("B"))],
        );
        for now_ms in [4_000, 10_000, 60_000] {
            let step = guest_step_with(&mut guest, now_ms, blocked.clone(), Vec::new());
            assert!(step.make.is_none());
            assert!(
                !step
                    .logs
                    .iter()
                    .any(|line| line.contains("requests are made anyway"))
            );
        }
    }
}

fn watching_guest_at(now_ms: u64) -> FollowReadings {
    let mut at = readings(JumpRole::Guest, now_ms);
    at.this_copy_watches = true;
    at
}

fn a_watching_guest_with_a_kept_request() -> FollowMachine {
    let mut guest = a_guest();
    guest.step(&watching_guest_at(5));
    assert_eq!(
        guest.on_game_request(&sub_phase("B"), &place("A"), 10),
        GameRequestVerdict::Swallow
    );
    guest
}

#[test]
fn a_watching_guest_keeps_its_request_and_never_asks_the_host() {
    let mut guest = a_watching_guest_with_a_kept_request();
    for now_ms in [20, 1_500, 25_000, 100_000] {
        let step = guest.step(&watching_guest_at(now_ms));
        assert!(asks_sent(&step).is_empty());
        assert!(step.make.is_none());
    }
}

#[test]
fn a_watching_guest_never_runs_its_request_after_the_give_up_time() {
    let mut guest = a_watching_guest_with_a_kept_request();
    let step = guest.step(&watching_guest_at(10 + STORY_ASK_GIVE_UP_MS + 1));
    assert!(step.make.is_none());
    assert!(!step.logs.iter().any(|line| line.contains("runs here")));
}

#[test]
fn a_watching_guest_says_once_then_every_ten_seconds_that_it_keeps_the_request() {
    let mut guest = a_guest();
    guest.step(&watching_guest_at(5));
    guest.on_game_request(&sub_phase("B"), &place("A"), 10);
    let first = guest.step(&watching_guest_at(20));
    assert!(
        first
            .logs
            .iter()
            .any(|line| line.contains("this copy watches, so its own request"))
    );
    let quiet = guest.step(&watching_guest_at(10 + STORY_WATCHING_KEPT_LOG_MS - 1));
    assert!(!quiet.logs.iter().any(|line| line.contains("still kept")));
    let due = guest.step(&watching_guest_at(10 + STORY_WATCHING_KEPT_LOG_MS));
    assert!(
        due.logs
            .iter()
            .any(|line| line.contains("still kept after 10000 ms"))
    );
}

#[test]
fn a_hosts_move_clears_a_watching_guests_kept_request() {
    let mut guest = a_watching_guest_with_a_kept_request();
    let mut told = watching_guest_at(100);
    told.moves.push(a_move(1, sub_phase("B")));
    let step = guest.step(&told);
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("heard the host's move 1"))
    );
    let later = guest.step(&watching_guest_at(50_000));
    assert!(!later.logs.iter().any(|line| line.contains("still kept")));
}

#[test]
fn a_place_change_drops_a_watching_guests_kept_request() {
    let mut guest = a_watching_guest_with_a_kept_request();
    let mut moved = watching_guest_at(100);
    moved.current_place = place("C");
    moved.host_place = place("C");
    let step = guest.step(&moved);
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("the place changed"))
    );
    let after = guest.step(&watching_guest_at(50_000));
    assert!(after.make.is_none());
}

#[test]
fn a_request_kept_while_watching_is_dropped_when_watching_ends() {
    let mut guest = a_watching_guest_with_a_kept_request();
    let step = guest.step(&readings(JumpRole::Guest, 100));
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("no longer watches"))
    );
    assert!(asks_sent(&step).is_empty());
}

#[test]
fn a_guest_that_does_not_watch_still_asks_and_gives_up_as_before() {
    let mut guest = a_guest();
    guest.on_game_request(&sub_phase("B"), &place("A"), 10);
    assert_eq!(asks_sent(&guest_at(&mut guest, 20)).len(), 1);
    let step = guest_at(&mut guest, 10 + STORY_ASK_GIVE_UP_MS);
    assert!(step.logs.iter().any(|line| line.contains("runs here")));
}

#[test]
fn the_host_refuses_an_ask_from_a_watching_guest_and_never_makes_it() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.partner_watches = true;
    at.asks.push(an_ask(7, "A", sub_phase("B")));
    let step = host.step(&at);
    assert!(step.make.is_none());
    assert!(moves_sent(&step).is_empty());
    assert_eq!(
        answers_sent(&step),
        vec![StoryAskAnswer {
            number: 7,
            verdict: AskVerdict::RefusedGuestWatches
        }]
    );
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("the guest watches and never leads"))
    );
}

#[test]
fn the_host_refuses_a_kept_ask_when_the_guest_starts_watching_before_it_is_made() {
    let mut host = a_host();
    let mut busy = readings(JumpRole::Host, 100);
    busy.readiness = not_ready();
    busy.asks.push(an_ask(7, "A", sub_phase("B")));
    assert!(answers_sent(&host.step(&busy)).is_empty());
    let mut watching = readings(JumpRole::Host, 200);
    watching.partner_watches = true;
    let step = host.step(&watching);
    assert!(step.make.is_none());
    assert_eq!(
        answers_sent(&step)[0].verdict,
        AskVerdict::RefusedGuestWatches
    );
}

#[test]
fn a_guest_that_does_not_watch_is_asked_as_before() {
    let mut host = a_host();
    let mut at = readings(JumpRole::Host, 100);
    at.asks.push(an_ask(7, "A", sub_phase("B")));
    assert_eq!(host.step(&at).make, Some(sub_phase("B")));
}
