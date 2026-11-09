use rc_game::fight_sync::{
    GuestFightStream, GuestNotice, GuestPop, GuestReceived, GuestRestart, HostFightStream, HostPop,
    HostRequestOut, HostStreamOut, MAX_WAITING_GUEST_REQUESTS, OWN_POP_HELD_FOR_HOST_COPY_MS,
    PARTNER_RESTART_EXPECTED_MS, PopSource, QUEUED_CALL_GIVEN_UP_AFTER_MS, QueueRefusal,
    QueuedCallNotice, REFUSED_REQUEST_RESEND_AFTER_MS, REFUSED_REQUEST_RESENDS_BEFORE_RUNNING_HERE,
    RefusedRequestStep, RequestReceipt, RestartWait, SlotState, StreamEntry, fight_windows_of_pack,
    fired_registry_index, is_player_tested_callback_kind,
};
use rc_game::story_relay::RELAY_CALL_QUEUE_TRIES;
use rc_proto::{
    FightRequest, FightRequestAnswer, FightRequestKind, FightRequestOutcome, FightRequestRefusal,
    Message, StoryCallArg, decode, encode,
};

const STONE_FIELD_PACK: &str = "A_STONE_FIELD_02";
const STONE_FIELD_PHASE: u16 = 23;
const FIGHT_ARM_POP: &str = "EID_0200_a0030g_Talk01";
const ATTACK_COUNT: &str = "AttackCount_Talk";
const GUEST_TRAP: &str = "Radio_Trap_01";
const OTHER_TRAP: &str = "Radio_Trap_02";
const MANAGER: u64 = 0x1_4122_D440;
const REQUEST_SLOT: u64 = 0x1_4130_2070;
const OTHER_SLOT: u64 = 0x1_4130_2818;
const NAME_IN_NO_WINDOW_LIST: &str = "ALL_MAGIC_LEARN";
const RUN36_DOUBLED_START: &str = "EID_0200_a0037g_02_Start";
const HERE: Option<u16> = Some(STONE_FIELD_PHASE);

fn over_the_wire(message: Message) -> Message {
    assert_eq!(message.validate(), Ok(()));
    assert!(message.is_relayed());
    decode(&encode(&message).unwrap()).unwrap()
}

fn request_over_the_wire(request: FightRequest) -> FightRequest {
    let Message::FightRequest(request) = over_the_wire(Message::FightRequest(request)) else {
        panic!("a fight request stays a fight request");
    };
    request
}

fn message_of(outs: &[HostStreamOut]) -> Message {
    let message = outs
        .iter()
        .find_map(HostStreamOut::message)
        .expect("a step the guest is sent");
    over_the_wire(message)
}

fn deliver(guest: &mut GuestFightStream, outs: &[HostStreamOut]) {
    for message in outs.iter().filter_map(HostStreamOut::message) {
        guest.receive_message(over_the_wire(message), HERE);
    }
}

fn host_handler_ends(host: &mut HostFightStream, guest: &mut GuestFightStream) {
    let ended: Vec<HostStreamOut> = host.script_slot(SlotState::Idle).into_iter().collect();
    deliver(guest, &ended);
}

fn open_fight() -> (HostFightStream, GuestFightStream) {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    let opened = host.popped(FIGHT_ARM_POP, Vec::new(), false);
    // The guest's own copy of the arming pop ran first, so it skips the host's entry 0.
    let mut guest = GuestFightStream::new();
    guest.pop(FIGHT_ARM_POP, PopSource::Fired, 0, HERE, false);
    let received = guest.receive_message(message_of(&opened), HERE);
    assert!(matches!(received, GuestReceived::Opened { .. }));
    deliver(&mut guest, &opened[2..]);
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::ArmingEntrySkipped { .. }]
    ));
    (host, guest)
}

fn trap_args() -> Vec<StoryCallArg> {
    vec![
        StoryCallArg::Int(4),
        StoryCallArg::Text(b"trap_gate".to_vec()),
    ]
}

fn guest_asks_with_no_host_copy(
    guest: &mut GuestFightStream,
    name: &str,
    args: Vec<StoryCallArg>,
    now_ms: u64,
) -> Vec<FightRequest> {
    assert_eq!(
        guest.pop_by_player(name, args, HERE, false, now_ms),
        GuestPop::HeldForHostCopy
    );
    guest.tick(
        SlotState::Busy,
        HERE,
        false,
        now_ms + OWN_POP_HELD_FOR_HOST_COPY_MS,
    );
    assert!(
        guest.take_requests().is_empty(),
        "held for the whole hold time"
    );
    guest.tick(
        SlotState::Busy,
        HERE,
        false,
        now_ms + OWN_POP_HELD_FOR_HOST_COPY_MS + 1,
    );
    guest.take_requests()
}

fn host_runs_its_queue_once(host: &mut HostFightStream) -> Vec<StreamEntry> {
    let mut streamed = Vec::new();
    while let Some(call) = host.tick_requests(SlotState::Idle, false) {
        host.request_queued_at(REQUEST_SLOT);
        assert_eq!(
            host.tick_requests(SlotState::Idle, false),
            None,
            "never two requests queued at once"
        );
        assert_eq!(
            host.judge_pop(&call.name, PopSource::Queued, REQUEST_SLOT),
            HostPop::RunRequested
        );
        for out in host.popped(&call.name, call.args, false) {
            if let HostStreamOut::Entry(entry) = out {
                streamed.push(entry);
            }
        }
    }
    streamed
}

fn outcomes(answers: &[FightRequestAnswer]) -> Vec<(String, u32, FightRequestOutcome)> {
    answers
        .iter()
        .map(|answer| (answer.name.clone(), answer.copy_number, answer.outcome))
        .collect()
}

fn refused(reason: FightRequestRefusal) -> FightRequestOutcome {
    FightRequestOutcome::Refused { reason }
}

fn answer_the_guest(guest: &mut GuestFightStream, answers: Vec<FightRequestAnswer>) {
    for answer in answers {
        let received =
            guest.receive_message(over_the_wire(Message::FightRequestAnswer(answer)), HERE);
        assert!(matches!(received, GuestReceived::RequestAnswer { .. }));
    }
}

fn guest_replays(
    host: &mut HostFightStream,
    guest: &mut GuestFightStream,
    streamed: Vec<StreamEntry>,
    now_ms: u64,
) -> Vec<String> {
    for entry in streamed {
        guest.receive_message(over_the_wire(Message::FightStream(entry.to_wire())), HERE);
    }
    host_handler_ends(host, guest);
    let mut ran = Vec::new();
    while let Some(entry) = guest.tick(SlotState::Idle, HERE, false, now_ms) {
        assert_eq!(
            guest.pop(&entry.name, PopSource::Queued, 0, HERE, false),
            GuestPop::RunReplayed { seq: entry.seq }
        );
        ran.push(entry.name);
    }
    ran
}

#[test]
fn a_guest_only_trap_is_not_run_on_the_guest_runs_once_on_the_host_and_comes_back_in_the_stream() {
    let (mut host, mut guest) = open_fight();
    let requests = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 100);
    assert_eq!(requests.len(), 1);
    assert_eq!(guest.take_requests(), Vec::new());
    let request = request_over_the_wire(requests[0].clone());
    assert!(matches!(
        &request.kind,
        FightRequestKind::RunPop { copy_number: 1, .. }
    ));
    assert_eq!(
        host.receive_request(request),
        HostRequestOut::Run(RequestReceipt::Waiting)
    );
    assert!(host.take_answers().is_empty(), "answered once it ran");
    let streamed = host_runs_its_queue_once(&mut host);
    assert_eq!(
        outcomes(&host.take_answers()),
        [(GUEST_TRAP.to_owned(), 1, FightRequestOutcome::Ran)]
    );
    assert_eq!(streamed.len(), 1);
    assert_eq!(streamed[0].name, GUEST_TRAP);
    assert_eq!(streamed[0].args, trap_args());
    assert_eq!(
        guest_replays(&mut host, &mut guest, streamed, 4_000),
        vec![GUEST_TRAP]
    );
    assert_eq!(guest.requests_unanswered(), 0);
    // The streamed run paired with the guest's pop, so a later touch is a new one.
    assert_eq!(
        guest.pop_by_player(GUEST_TRAP, trap_args(), HERE, false, 4_100),
        GuestPop::HeldForHostCopy
    );
    assert!(guest.take_requests().is_empty());
}

#[test]
fn every_touch_while_another_is_held_is_counted_and_becomes_its_own_request() {
    // F17 2: a repeat touch while one was held used to be dropped uncounted.
    let (mut host, mut guest) = open_fight();
    assert_eq!(
        guest.pop_by_player(GUEST_TRAP, trap_args(), HERE, false, 0),
        GuestPop::HeldForHostCopy
    );
    assert_eq!(
        guest.pop_by_player(GUEST_TRAP, trap_args(), HERE, false, 50),
        GuestPop::HeldForHostCopy
    );
    guest.tick(
        SlotState::Busy,
        HERE,
        false,
        OWN_POP_HELD_FOR_HOST_COPY_MS + 51,
    );
    let requests = guest.take_requests();
    let copies: Vec<u32> = requests
        .iter()
        .map(|request| match &request.kind {
            FightRequestKind::RunPop { copy_number, .. } => *copy_number,
            FightRequestKind::RestartPhase => 0,
        })
        .collect();
    assert_eq!(copies, [1, 2]);
    assert_eq!(guest.requests_unanswered(), 2);
    for request in requests {
        assert_eq!(
            host.receive_request(request_over_the_wire(request)),
            HostRequestOut::Run(RequestReceipt::Waiting)
        );
    }
    let streamed = host_runs_its_queue_once(&mut host);
    assert_eq!(streamed.len(), 2, "both touches run on the host");
    assert_eq!(
        guest_replays(&mut host, &mut guest, streamed, 4_000),
        vec![GUEST_TRAP, GUEST_TRAP]
    );
    answer_the_guest(&mut guest, host.take_answers());
    assert_eq!(guest.requests_unanswered(), 0);
}

#[test]
fn when_both_games_pop_it_and_the_host_streamed_first_the_guest_sends_nothing() {
    let (mut host, mut guest) = open_fight();
    assert_eq!(
        host.judge_pop(GUEST_TRAP, PopSource::Fired, 0),
        HostPop::Run
    );
    let streamed = host.popped(GUEST_TRAP, trap_args(), false);
    guest.tick(SlotState::Busy, HERE, false, 10);
    deliver(&mut guest, &streamed);
    assert_eq!(
        guest.pop_by_player(GUEST_TRAP, trap_args(), HERE, false, 20),
        GuestPop::Hold
    );
    assert!(guest.take_requests().is_empty());
    assert_eq!(
        guest_replays(&mut host, &mut guest, Vec::new(), 30),
        vec![GUEST_TRAP]
    );
}

#[test]
fn when_both_games_pop_it_and_the_request_arrives_after_the_host_ran_its_own_it_is_not_run() {
    let (mut host, mut guest) = open_fight();
    let request = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0).remove(0);
    assert_eq!(
        host.judge_pop(GUEST_TRAP, PopSource::Fired, 0),
        HostPop::Run
    );
    let own = host.popped(GUEST_TRAP, trap_args(), false);
    assert_eq!(
        host.receive_request(request),
        HostRequestOut::Run(RequestReceipt::AlreadyRanHere)
    );
    assert_eq!(
        outcomes(&host.take_answers()),
        [(
            GUEST_TRAP.to_owned(),
            1,
            refused(FightRequestRefusal::AlreadyStreamed)
        )]
    );
    assert!(host_runs_its_queue_once(&mut host).is_empty());
    deliver(&mut guest, &own);
    assert_eq!(
        guest.requests_unanswered(),
        0,
        "the host's own copy answered it"
    );
    assert_eq!(
        guest_replays(&mut host, &mut guest, Vec::new(), 4_000),
        vec![GUEST_TRAP]
    );
}

#[test]
fn run36_the_guests_own_copy_five_seconds_after_the_hosts_entry_pairs_with_it_and_is_no_request() {
    // Run36: the guest replayed the host's a0037g_02_Start 3.3 s after it came, its own game
    // queued the same start 2 s later, and the 3 s echo span had run out: it ran twice.
    let (mut host, mut guest) = open_fight();
    let streamed = host.popped(RUN36_DOUBLED_START, Vec::new(), false);
    guest.tick(SlotState::Busy, HERE, false, 0);
    deliver(&mut guest, &streamed);
    assert_eq!(
        guest_replays(&mut host, &mut guest, Vec::new(), 3_300),
        vec![RUN36_DOUBLED_START]
    );
    assert_eq!(
        guest.pop_by_player(RUN36_DOUBLED_START, Vec::new(), HERE, false, 5_300),
        GuestPop::Hold
    );
    guest.tick(SlotState::Busy, HERE, false, 60_000);
    assert!(guest.take_requests().is_empty());
    assert!(
        !guest
            .take_notices()
            .iter()
            .any(|notice| matches!(notice, GuestNotice::RequestSent { .. }))
    );
}

#[test]
fn a_player_pop_whose_host_copy_comes_within_the_hold_pairs_with_it_and_is_no_request() {
    let (mut host, mut guest) = open_fight();
    guest.tick(SlotState::Busy, HERE, false, 0);
    assert_eq!(
        guest.pop_by_player(GUEST_TRAP, trap_args(), HERE, false, 0),
        GuestPop::HeldForHostCopy
    );
    let streamed = host.popped(GUEST_TRAP, trap_args(), false);
    guest.tick(SlotState::Busy, HERE, false, 1_000);
    deliver(&mut guest, &streamed);
    assert_eq!(
        guest.take_notices(),
        [GuestNotice::PlayerPopPairedWithHostCopy {
            name: GUEST_TRAP.to_owned(),
            held_ms: 1_000
        }]
    );
    guest.tick(SlotState::Busy, HERE, false, 60_000);
    assert!(guest.take_requests().is_empty());
}

#[test]
fn a_request_runs_only_when_the_hosts_stream_carried_no_further_copy_since_the_guests_count() {
    let (mut host, mut guest) = open_fight();
    guest.tick(SlotState::Busy, HERE, false, 0);
    deliver(&mut guest, &host.popped(GUEST_TRAP, trap_args(), false));
    assert_eq!(
        guest.pop_by_player(GUEST_TRAP, trap_args(), HERE, false, 10),
        GuestPop::Hold,
        "the first own copy pairs with the host's entry"
    );
    let request = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 20).remove(0);
    assert!(matches!(
        &request.kind,
        FightRequestKind::RunPop { copy_number: 2, .. }
    ));
    // The host's stream carries the name once more before the request comes: that answered it.
    host.popped(GUEST_TRAP, trap_args(), false);
    assert_eq!(host.streamed_copies(GUEST_TRAP), 2);
    assert_eq!(
        host.receive_request(request.clone()),
        HostRequestOut::Run(RequestReceipt::AlreadyRanHere)
    );
    let mut counted_both = request;
    if let FightRequestKind::RunPop { copy_number, .. } = &mut counted_both.kind {
        *copy_number = 3;
    }
    assert_eq!(
        host.receive_request(counted_both),
        HostRequestOut::Run(RequestReceipt::Waiting)
    );
}

#[test]
fn when_the_host_pops_its_own_copy_while_the_request_is_queued_the_request_does_not_run() {
    let (mut host, mut guest) = open_fight();
    let request = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0).remove(0);
    host.receive_request(request);
    let queued = host.tick_requests(SlotState::Idle, false).unwrap();
    host.request_queued_at(REQUEST_SLOT);
    // A fired callback beats a queued call: the host's own trap pops first.
    assert_eq!(
        host.judge_pop(GUEST_TRAP, PopSource::Fired, 0),
        HostPop::Run
    );
    let own = host.popped(GUEST_TRAP, trap_args(), false);
    assert_eq!(
        host.judge_pop(&queued.name, PopSource::Queued, REQUEST_SLOT),
        HostPop::HoldSuperseded
    );
    assert!(!host.is_request_queued());
    assert_eq!(
        outcomes(&host.take_answers()),
        [(
            GUEST_TRAP.to_owned(),
            1,
            refused(FightRequestRefusal::AlreadyStreamed)
        )]
    );
    deliver(&mut guest, &own);
    assert_eq!(
        guest_replays(&mut host, &mut guest, Vec::new(), 4_000),
        vec![GUEST_TRAP]
    );
}

#[test]
fn every_host_pop_of_the_name_after_it_ran_the_request_is_a_real_pop_that_runs_and_streams() {
    // F17 3: the host used to hold its next own copy of a name it ran for a request, however late
    // in the epoch, and the guest then never got that pop.
    let (mut host, mut guest) = open_fight();
    let request = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0).remove(0);
    host.receive_request(request);
    assert_eq!(host_runs_its_queue_once(&mut host).len(), 1);
    for _ in 0..2 {
        assert_eq!(
            host.judge_pop(GUEST_TRAP, PopSource::Fired, 0),
            HostPop::Run
        );
        assert!(
            host.popped(GUEST_TRAP, trap_args(), false)
                .iter()
                .any(|out| matches!(out, HostStreamOut::Entry(_)))
        );
    }
    assert_eq!(host.streamed_copies(GUEST_TRAP), 3);
}

#[test]
fn a_queued_pop_of_the_requests_name_before_its_slot_is_known_is_the_hosts_own() {
    let (mut host, mut guest) = open_fight();
    let request = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0).remove(0);
    host.receive_request(request);
    host.tick_requests(SlotState::Idle, false).unwrap();
    assert_eq!(
        host.judge_pop(GUEST_TRAP, PopSource::Queued, OTHER_SLOT),
        HostPop::Run,
        "a request is known by its slot, never by its name"
    );
    assert!(host.is_request_queued());
}

#[test]
fn requests_queue_one_at_a_time_after_other_calls_and_a_lost_one_is_queued_again() {
    let (mut host, mut guest) = open_fight();
    let mut requests = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0);
    requests.extend(guest_asks_with_no_host_copy(
        &mut guest,
        OTHER_TRAP,
        Vec::new(),
        0,
    ));
    for request in requests {
        host.receive_request(request);
    }
    assert_eq!(host.requests_waiting(), 2);
    assert_eq!(host.tick_requests(SlotState::Idle, true), None);
    assert_eq!(host.tick_requests(SlotState::Busy, false), None);
    let first = host.tick_requests(SlotState::Idle, false).unwrap();
    assert_eq!(first.name, GUEST_TRAP);
    assert_eq!(host.tick_requests(SlotState::Idle, false), None);
    assert_eq!(host.request_call_lost().as_deref(), Some(GUEST_TRAP));
    assert_eq!(host.tick_requests(SlotState::Idle, false), Some(first));
    host.request_queued_at(REQUEST_SLOT);
    assert_eq!(
        host.judge_pop(GUEST_TRAP, PopSource::Queued, REQUEST_SLOT),
        HostPop::RunRequested
    );
    assert_eq!(host.tick_requests(SlotState::Busy, false), None);
    assert_eq!(
        host.tick_requests(SlotState::Idle, false).unwrap().name,
        OTHER_TRAP
    );
}

#[test]
fn a_request_for_an_old_epoch_or_outside_the_window_is_not_run() {
    let (mut host, mut guest) = open_fight();
    let request = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0).remove(0);
    host.popped(ATTACK_COUNT, Vec::new(), false);
    let old_epoch = host.epoch();
    host.restart();
    assert_eq!(host.receive_request(request.clone()), HostRequestOut::Stale);
    let answers = host.take_answers();
    assert_eq!(
        outcomes(&answers),
        [(
            GUEST_TRAP.to_owned(),
            1,
            refused(FightRequestRefusal::FightMovedOn)
        )]
    );
    assert_eq!(
        answers[0].epoch, old_epoch,
        "answered in the epoch it asked in"
    );
    let mut closed = HostFightStream::new();
    closed.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    assert_eq!(closed.receive_request(request), HostRequestOut::NotInWindow);
    assert_eq!(closed.take_answers().len(), 1);
}

#[test]
fn requests_waiting_when_the_epoch_ends_are_answered_in_the_epoch_they_were_asked_in() {
    let (mut host, mut guest) = open_fight();
    let epoch = host.epoch();
    for request in guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0) {
        host.receive_request(request);
    }
    host.restart();
    let answers = host.take_answers();
    assert_eq!(
        outcomes(&answers),
        [(
            GUEST_TRAP.to_owned(),
            1,
            refused(FightRequestRefusal::FightMovedOn)
        )]
    );
    assert_eq!(
        (answers[0].phase, answers[0].epoch),
        (STONE_FIELD_PHASE, epoch)
    );
}

#[test]
fn player_local_and_outside_window_pops_are_never_requests() {
    let (_, mut guest) = open_fight();
    assert_eq!(
        guest.pop_by_player("SQ_Player_Receive_Damage", Vec::new(), HERE, false, 0),
        GuestPop::RunPlayerLocal
    );
    assert_eq!(
        guest.pop_by_player(GUEST_TRAP, Vec::new(), HERE, true, 0),
        GuestPop::RunPlayerLocal
    );
    assert_eq!(
        guest.pop_by_player(
            GUEST_TRAP,
            Vec::new(),
            Some(STONE_FIELD_PHASE + 1),
            false,
            0
        ),
        GuestPop::Outside
    );
    assert!(guest.take_requests().is_empty());
}

#[test]
fn only_registry_kinds_tested_against_this_player_count_as_player_pops() {
    for kind in [4, 5, 6, 7, 8, 0xA, 0x11] {
        assert!(is_player_tested_callback_kind(kind));
    }
    for kind in [-1, 0, 1, 2, 3, 9, 0xE, 0x10, 0x14] {
        assert!(!is_player_tested_callback_kind(kind));
    }
    let entry = |index: u64| MANAGER + 0x70 + index * 0x7C8;
    assert_eq!(fired_registry_index(MANAGER, entry(0)), Some(0));
    assert_eq!(fired_registry_index(MANAGER, entry(255)), Some(255));
    assert_eq!(fired_registry_index(MANAGER, entry(256)), None);
    assert_eq!(fired_registry_index(MANAGER, entry(3) + 8), None);
    assert_eq!(fired_registry_index(MANAGER, MANAGER), None);
}

#[test]
fn guest_restarts_first_and_the_host_restart_resumes_the_replay_at_the_new_epoch() {
    let (mut host, mut guest) = open_fight();
    let before = host.popped(ATTACK_COUNT, Vec::new(), false);
    deliver(&mut guest, &before);
    host_handler_ends(&mut host, &mut guest);
    assert!(guest.tick(SlotState::Idle, HERE, false, 0).is_some());
    assert_eq!(guest.own_restart(100), GuestRestart::WaitingForHost);
    assert_eq!(guest.pending_len(), 0);
    let tail = host.popped(ATTACK_COUNT, Vec::new(), false);
    deliver(&mut guest, &tail);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 200), None);
    let restarted = host.restart();
    let received = guest.receive_message(message_of(&restarted), HERE);
    let GuestReceived::Restarted(GuestRestart::Paired { epoch }) = received else {
        panic!("expected both restarts paired, got {received:?}");
    };
    assert_eq!(epoch, host.epoch());
    assert_eq!(guest.restart_wait(), RestartWait::None);
    let after = host.popped(ATTACK_COUNT, Vec::new(), false);
    let streamed: Vec<StreamEntry> = after
        .into_iter()
        .filter_map(|out| match out {
            HostStreamOut::Entry(entry) => Some(entry),
            _ => None,
        })
        .collect();
    assert_eq!((streamed[0].epoch, streamed[0].seq), (epoch, 0));
    assert_eq!(
        guest_replays(&mut host, &mut guest, streamed, 300),
        vec![ATTACK_COUNT]
    );
    assert!(guest.take_requests().is_empty());
}

#[test]
fn host_restarts_first_and_the_replay_waits_for_the_guests_own_restart() {
    let (mut host, mut guest) = open_fight();
    host.popped(ATTACK_COUNT, Vec::new(), false);
    let restarted = host.restart();
    guest.tick(SlotState::Idle, HERE, false, 1_000);
    let received = guest.receive_message(message_of(&restarted), HERE);
    assert!(matches!(
        received,
        GuestReceived::Restarted(GuestRestart::WaitingForOwnRestart { .. })
    ));
    let after = host.popped(ATTACK_COUNT, Vec::new(), false);
    deliver(&mut guest, &after);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 2_000), None);
    assert!(matches!(
        guest.own_restart(2_500),
        GuestRestart::Paired { .. }
    ));
    assert_eq!(
        guest_replays(&mut host, &mut guest, Vec::new(), 2_600),
        vec![ATTACK_COUNT]
    );
}

#[test]
fn a_host_restart_the_guest_never_follows_lets_the_replay_go_on_after_the_wait() {
    let (mut host, mut guest) = open_fight();
    let restarted = host.restart();
    guest.receive_message(message_of(&restarted), HERE);
    let after = host.popped(ATTACK_COUNT, Vec::new(), false);
    deliver(&mut guest, &after);
    host_handler_ends(&mut host, &mut guest);
    assert_eq!(
        guest.tick(SlotState::Idle, HERE, false, PARTNER_RESTART_EXPECTED_MS),
        None
    );
    let entry = guest.tick(
        SlotState::Idle,
        HERE,
        false,
        PARTNER_RESTART_EXPECTED_MS + 1,
    );
    assert_eq!(entry.map(|entry| entry.name).as_deref(), Some(ATTACK_COUNT));
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::HostRestartedAlone { .. }]
    ));
}

#[test]
fn a_guest_only_restart_asks_the_host_to_restart_once_and_both_go_on_at_the_new_epoch() {
    let (mut host, mut guest) = open_fight();
    assert_eq!(guest.own_restart(0), GuestRestart::WaitingForHost);
    assert_eq!(guest.own_restart(500), GuestRestart::SameRestart);
    assert_eq!(
        guest.tick(SlotState::Idle, HERE, false, PARTNER_RESTART_EXPECTED_MS),
        None
    );
    assert!(guest.take_requests().is_empty());
    guest.tick(
        SlotState::Idle,
        HERE,
        false,
        PARTNER_RESTART_EXPECTED_MS + 1,
    );
    guest.tick(
        SlotState::Idle,
        HERE,
        false,
        PARTNER_RESTART_EXPECTED_MS + 100,
    );
    let requests = guest.take_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].kind, FightRequestKind::RestartPhase);
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::RestartRequested { .. }]
    ));
    let request = request_over_the_wire(requests[0].clone());
    assert_eq!(
        host.receive_request(request.clone()),
        HostRequestOut::RestartPhase
    );
    // The hook restarts the host's phase and the stream bumps the epoch.
    let restarted = host.restart();
    assert_eq!(host.receive_request(request), HostRequestOut::Stale);
    let received = guest.receive_message(message_of(&restarted), HERE);
    assert!(matches!(
        received,
        GuestReceived::Restarted(GuestRestart::Paired { .. })
    ));
    let after = host.popped(ATTACK_COUNT, Vec::new(), false);
    deliver(&mut guest, &after);
    assert_eq!(
        guest_replays(&mut host, &mut guest, Vec::new(), 7_000),
        vec![ATTACK_COUNT]
    );
}

#[test]
fn a_guest_whose_restart_request_is_never_answered_leaves_the_stream() {
    let (_, mut guest) = open_fight();
    guest.own_restart(0);
    guest.tick(
        SlotState::Idle,
        HERE,
        false,
        PARTNER_RESTART_EXPECTED_MS + 1,
    );
    guest.take_notices();
    guest.tick(
        SlotState::Idle,
        HERE,
        false,
        2 * PARTNER_RESTART_EXPECTED_MS + 1,
    );
    assert!(!guest.is_active_in(HERE));
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::LeftWithoutHostRestart { .. }]
    ));
}

#[test]
fn the_host_closing_the_fight_while_the_guest_waits_for_its_restart_leaves_the_stream() {
    let (mut host, mut guest) = open_fight();
    guest.own_restart(0);
    let mut closing = host.popped(ATTACK_COUNT, Vec::new(), false);
    closing.extend(host.popped("EID_0200_a0080_End", Vec::new(), false));
    for out in &closing {
        if let Some(message) = out.message() {
            guest.receive_message(over_the_wire(message), HERE);
        }
    }
    assert!(!guest.is_active_in(HERE));
}

#[test]
fn a_restart_outside_a_joined_fight_changes_nothing() {
    let mut guest = GuestFightStream::new();
    assert_eq!(guest.own_restart(0), GuestRestart::NotInFight);
    assert!(guest.take_requests().is_empty());
}

fn queue_one_request(host: &mut HostFightStream, guest: &mut GuestFightStream, name: &str) {
    for request in guest_asks_with_no_host_copy(guest, name, Vec::new(), 0) {
        host.receive_request(request_over_the_wire(request));
    }
    let call = host.tick_requests(SlotState::Idle, false).unwrap();
    assert_eq!(call.name, name);
    host.request_queued_at(REQUEST_SLOT);
}

fn run_pop(host: &HostFightStream, name: &str) -> FightRequest {
    FightRequest {
        phase: STONE_FIELD_PHASE,
        epoch: host.epoch(),
        kind: FightRequestKind::RunPop {
            name: name.to_owned(),
            args: Vec::new(),
            copy_number: host.streamed_copies(name) + 1,
        },
    }
}

#[test]
fn a_request_still_in_the_queue_when_the_fight_restarts_blocks_every_other_call_until_it_leaves() {
    // F7a 1: clearing the requests on a restart used to leave the hook's slot record set for good,
    // which stalled every relay call of the host.
    let (mut host, mut guest) = open_fight();
    queue_one_request(&mut host, &mut guest, GUEST_TRAP);
    assert!(host.has_own_call_queued());
    host.popped(ATTACK_COUNT, Vec::new(), false);
    assert!(matches!(
        host.restart().as_slice(),
        [HostStreamOut::Restarted { .. }]
    ));
    assert!(!host.is_request_queued());
    assert_eq!(host.earlier_calls_queued(), 1);
    assert!(
        host.has_own_call_queued(),
        "still in the engine queue, so nothing goes beside it"
    );
    let mut fresh = guest_asks_with_no_host_copy(&mut guest, OTHER_TRAP, Vec::new(), 100).remove(0);
    fresh.epoch = host.epoch();
    host.receive_request(fresh);
    assert_eq!(host.tick_requests(SlotState::Idle, false), None);
    let mut asked = Vec::new();
    let notices = host.check_queued_calls(SlotState::Idle, 130, |name, arguments| {
        asked.push((name.to_owned(), arguments));
        true
    });
    assert!(notices.is_empty());
    assert_eq!(asked, [(GUEST_TRAP.to_owned(), REQUEST_SLOT)]);
    assert!(
        host.check_queued_calls(SlotState::Idle, 140, |_, _| false)
            .is_empty()
    );
    assert!(!host.has_own_call_queued());
    assert_eq!(
        host.tick_requests(SlotState::Idle, false).unwrap().name,
        OTHER_TRAP
    );
}

#[test]
fn an_earlier_request_that_pops_after_the_window_closed_is_known_and_stops_blocking() {
    let (mut host, mut guest) = open_fight();
    queue_one_request(&mut host, &mut guest, GUEST_TRAP);
    host.popped(
        fight_windows_of_pack(STONE_FIELD_PACK)
            .next()
            .unwrap()
            .end_ids[0],
        Vec::new(),
        false,
    );
    assert!(host.open_window().is_none());
    assert!(host.has_own_call_queued());
    assert!(!host.earlier_request_popped(GUEST_TRAP, OTHER_SLOT));
    assert!(!host.earlier_request_popped(OTHER_TRAP, REQUEST_SLOT));
    assert!(host.earlier_request_popped(GUEST_TRAP, REQUEST_SLOT));
    assert!(!host.has_own_call_queued());
}

#[test]
fn a_queued_request_is_given_up_after_the_limit_with_menu_time_not_counted() {
    // F7a 13: a request queued on the host had no limit.
    let (mut host, mut guest) = open_fight();
    queue_one_request(&mut host, &mut guest, GUEST_TRAP);
    assert!(
        host.check_queued_calls(SlotState::Idle, 0, |_, _| true)
            .is_empty()
    );
    assert!(
        host.check_queued_calls(SlotState::SteppingHeld, 120_000, |_, _| true)
            .is_empty(),
        "a held slot (a menu) does not count"
    );
    assert!(
        host.check_queued_calls(
            SlotState::Busy,
            120_000 + QUEUED_CALL_GIVEN_UP_AFTER_MS,
            |_, _| true
        )
        .is_empty()
    );
    assert_eq!(
        host.check_queued_calls(
            SlotState::Busy,
            120_000 + QUEUED_CALL_GIVEN_UP_AFTER_MS + 1,
            |_, _| true
        ),
        [QueuedCallNotice::GaveUp {
            name: GUEST_TRAP.to_owned(),
            waited_ms: QUEUED_CALL_GIVEN_UP_AFTER_MS + 1
        }]
    );
    assert!(!host.has_own_call_queued());
    assert!(!host.is_request_queued());
}

#[test]
fn a_request_whose_slot_lost_it_is_queued_again() {
    let (mut host, mut guest) = open_fight();
    queue_one_request(&mut host, &mut guest, GUEST_TRAP);
    assert_eq!(
        host.check_queued_calls(SlotState::Idle, 30, |_, _| false),
        [QueuedCallNotice::LeftQueue {
            name: GUEST_TRAP.to_owned()
        }]
    );
    assert_eq!(
        host.tick_requests(SlotState::Idle, false).unwrap().name,
        GUEST_TRAP
    );
}

#[test]
fn the_queued_request_is_matched_by_its_slot_and_name() {
    // F7a 8: a same-name call in another slot is the host's own copy, not the request.
    let (mut host, mut guest) = open_fight();
    queue_one_request(&mut host, &mut guest, GUEST_TRAP);
    assert_eq!(
        host.judge_pop(GUEST_TRAP, PopSource::Queued, OTHER_SLOT),
        HostPop::Run
    );
    assert!(
        host.is_request_queued(),
        "the host's own copy left it queued"
    );
    host.popped(GUEST_TRAP, Vec::new(), false);
    assert_eq!(
        host.judge_pop(GUEST_TRAP, PopSource::Queued, REQUEST_SLOT),
        HostPop::HoldSuperseded,
        "the stream already carried the copy the guest asked for"
    );
}

#[test]
fn only_window_functions_or_names_the_host_streamed_are_kept_and_at_most_the_cap_wait() {
    // F7a 12: the guest's request names were unchecked and the waiting list unbounded.
    let (mut host, _guest) = open_fight();
    assert_eq!(
        host.receive_request(run_pop(&host, NAME_IN_NO_WINDOW_LIST)),
        HostRequestOut::Run(RequestReceipt::NotAWindowFunction)
    );
    host.popped(NAME_IN_NO_WINDOW_LIST, Vec::new(), false);
    assert_eq!(
        host.receive_request(run_pop(&host, NAME_IN_NO_WINDOW_LIST)),
        HostRequestOut::Run(RequestReceipt::Waiting)
    );
    let window = fight_windows_of_pack(STONE_FIELD_PACK).next().unwrap();
    let mut kept = 1;
    for name in window
        .window_functions
        .iter()
        .filter(|name| !window.is_player_local_handler(name))
    {
        let out = host.receive_request(run_pop(&host, name));
        if kept < MAX_WAITING_GUEST_REQUESTS {
            assert_eq!(out, HostRequestOut::Run(RequestReceipt::Waiting), "{name}");
            kept += 1;
        } else {
            assert_eq!(out, HostRequestOut::Run(RequestReceipt::TooManyWaiting));
            break;
        }
    }
    assert_eq!(host.requests_waiting(), MAX_WAITING_GUEST_REQUESTS);
    let answers = host.take_answers();
    assert_eq!(
        answers
            .iter()
            .map(|answer| answer.outcome)
            .collect::<Vec<_>>(),
        [
            refused(FightRequestRefusal::NotAWindowFunction),
            refused(FightRequestRefusal::Full)
        ]
    );
}

#[test]
fn a_refused_queueing_is_retried_then_dropped_after_the_try_limit() {
    let (mut host, mut guest) = open_fight();
    for request in guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0) {
        host.receive_request(request);
    }
    for tries in 1..RELAY_CALL_QUEUE_TRIES {
        assert!(host.tick_requests(SlotState::Idle, false).is_some());
        assert_eq!(
            host.request_queue_refused(),
            Some(QueueRefusal::Again { tries })
        );
    }
    assert!(host.tick_requests(SlotState::Idle, false).is_some());
    assert_eq!(
        host.request_queue_refused(),
        Some(QueueRefusal::GaveUp {
            name: GUEST_TRAP.to_owned(),
            tries: RELAY_CALL_QUEUE_TRIES
        })
    );
    assert_eq!(host.tick_requests(SlotState::Idle, false), None);
    assert_eq!(
        outcomes(&host.take_answers()),
        [(
            GUEST_TRAP.to_owned(),
            1,
            refused(FightRequestRefusal::GaveUp)
        )]
    );
}

fn refuse_once(
    guest: &mut GuestFightStream,
    host: &HostFightStream,
    reason: FightRequestRefusal,
) -> GuestReceived {
    guest.receive_message(
        over_the_wire(Message::FightRequestAnswer(FightRequestAnswer {
            phase: STONE_FIELD_PHASE,
            epoch: host.epoch(),
            name: GUEST_TRAP.to_owned(),
            copy_number: 1,
            outcome: refused(reason),
        })),
        HERE,
    )
}

#[test]
fn a_request_refused_as_full_is_resent_after_the_wait_then_runs_here_after_the_last_resend() {
    // F17 1: a refused request was only logged on the host and blocked its name for the epoch.
    let (host, mut guest) = open_fight();
    guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0);
    let mut now = OWN_POP_HELD_FOR_HOST_COPY_MS + 1;
    guest.take_notices();
    for resend in 1..=REFUSED_REQUEST_RESENDS_BEFORE_RUNNING_HERE {
        assert_eq!(
            refuse_once(&mut guest, &host, FightRequestRefusal::Full),
            GuestReceived::RequestAnswer { kept: true }
        );
        guest.tick(
            SlotState::Busy,
            HERE,
            false,
            now + REFUSED_REQUEST_RESEND_AFTER_MS - 1,
        );
        assert!(guest.take_requests().is_empty(), "not before the wait");
        now += REFUSED_REQUEST_RESEND_AFTER_MS;
        guest.tick(SlotState::Busy, HERE, false, now);
        let resent = guest.take_requests();
        assert_eq!(resent.len(), 1);
        assert!(matches!(
            &resent[0].kind,
            FightRequestKind::RunPop { copy_number: 1, .. }
        ));
        assert!(guest.take_notices().iter().any(|notice| matches!(
            notice,
            GuestNotice::RequestResent { resend: got, .. } if *got == resend
        )));
    }
    refuse_once(&mut guest, &host, FightRequestRefusal::GaveUp);
    assert!(guest.take_notices().iter().any(|notice| matches!(
        notice,
        GuestNotice::RequestRefused {
            then: RefusedRequestStep::RunsHere,
            ..
        }
    )));
    assert_eq!(guest.requests_unanswered(), 0);
    guest.tick(SlotState::Busy, HERE, false, now + 60_000);
    assert!(guest.take_requests().is_empty(), "never sent a fifth time");
    let call = guest.refused_pop_to_run_here(SlotState::Idle).unwrap();
    assert_eq!(
        (call.name.as_str(), call.args.clone()),
        (GUEST_TRAP, trap_args())
    );
}

#[test]
fn a_request_that_is_no_function_of_the_hosts_fight_runs_here_as_the_players_own() {
    let (host, mut guest) = open_fight();
    guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0);
    refuse_once(&mut guest, &host, FightRequestRefusal::NotAWindowFunction);
    assert_eq!(guest.requests_unanswered(), 0);
    assert_eq!(guest.refused_pop_to_run_here(SlotState::Busy), None);
    let call = guest.refused_pop_to_run_here(SlotState::Idle).unwrap();
    guest.refused_pop_queued_at(call.clone(), None);
    let again = guest.refused_pop_to_run_here(SlotState::Idle).unwrap();
    assert_eq!(again, call, "a queueing the engine refused is tried again");
    guest.refused_pop_queued_at(again, Some(OTHER_SLOT));
    assert_eq!(guest.refused_pop_to_run_here(SlotState::Idle), None);
    assert_eq!(
        guest.pop(GUEST_TRAP, PopSource::Queued, REQUEST_SLOT, HERE, false),
        GuestPop::Hold,
        "only the slot it took is the run here"
    );
    assert_eq!(
        guest.pop(GUEST_TRAP, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::RunRefusedHere
    );
}

#[test]
fn after_a_refusal_the_next_touch_of_the_same_name_is_a_new_request() {
    let (host, mut guest) = open_fight();
    guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0);
    refuse_once(&mut guest, &host, FightRequestRefusal::NotAWindowFunction);
    let requests = guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 10_000);
    assert_eq!(requests.len(), 1);
    assert!(matches!(
        &requests[0].kind,
        FightRequestKind::RunPop { copy_number: 2, .. }
    ));
}

#[test]
fn an_answer_for_another_epoch_or_a_request_no_longer_waiting_is_not_used() {
    let (mut host, mut guest) = open_fight();
    guest_asks_with_no_host_copy(&mut guest, GUEST_TRAP, trap_args(), 0);
    let mut stale = FightRequestAnswer {
        phase: STONE_FIELD_PHASE,
        epoch: host.epoch() + 1,
        name: GUEST_TRAP.to_owned(),
        copy_number: 1,
        outcome: refused(FightRequestRefusal::NotAWindowFunction),
    };
    answer_the_guest(&mut guest, vec![stale.clone()]);
    assert_eq!(guest.requests_unanswered(), 1);
    stale.epoch = host.epoch();
    stale.copy_number = 2;
    answer_the_guest(&mut guest, vec![stale]);
    assert_eq!(guest.requests_unanswered(), 1);
    // The host's stream pairs with the request before the host's answer comes.
    guest.tick(SlotState::Busy, HERE, false, 4_000);
    deliver(&mut guest, &host.popped(GUEST_TRAP, trap_args(), false));
    assert_eq!(guest.requests_unanswered(), 0);
    assert_eq!(
        refuse_once(&mut guest, &host, FightRequestRefusal::NotAWindowFunction),
        GuestReceived::RequestAnswer { kept: false }
    );
    assert_eq!(guest.refused_pop_to_run_here(SlotState::Idle), None);
}
