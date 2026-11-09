use std::collections::VecDeque;

use rc_game::callback_gate::{EndFollow, EndQueue, ParkedCall, ParkedKind, QueueEvent};
use rc_game::fight_sync::{
    BASE_PLAYER_LOCAL_HANDLERS, CALLBACK_REGISTRY_ENTRY_STRIDE, GuestArrival, GuestFightStream,
    GuestOpen, GuestPop, GuestReceived, HostFightStream, HostStreamOut, InFlight,
    PARTNER_CALL_POP_EXPECTED_MS, PopSource, SCRIPT_SLOT_STEPPING_HELD_STATE, SlotState,
    WindowCloseCause, callback_registry_kind_address, pop_source, script_slot_state,
};
use rc_game::script_runner::{QUEUE_FIRST_SLOT_OFFSET, QUEUE_SLOT_STRIDE};
use rc_game::story_relay::QUEUED_CALL_ARGUMENTS_OFFSET;
use rc_proto::{Message, StoryCall, StoryCallArg, decode, encode};

const STONE_FIELD_PACK: &str = "A_STONE_FIELD_02";
const STONE_FIELD_PHASE: u16 = 23;
const FIGHT_START: &str = "EID_0200_a0030g_Start";
const FIGHT_END: &str = "EID_0200_a0080_End";
const ATTACK_COUNT: &str = "AttackCount_Talk";
const KILL_COUNT_END: &str = "EID_0200_a0037g_02_End";
const KILL_COUNT_MESSAGE: i32 = 212;
const BATTLE_PHASE_MESSAGE: i32 = 241;
const OWN_DAMAGE: &str = "SQ_Player_Receive_Damage";
const RADIO_HANDLER_OF_MESSAGE_210: &str = "Talk_Radio_GameOver";
const EARLIER_PARTNER_END: &str = "EID_0200_a0030g_Talk01_End";
const MANAGER: u64 = 0x1_4122_D440;
const TICK_MS: u64 = 33;

type Pop = (&'static str, Option<i32>);

struct HostRun {
    messages: Vec<Message>,
    streamed: Vec<(String, Vec<StoryCallArg>)>,
    closed_by: Option<WindowCloseCause>,
}

fn run34_fight_pops() -> Vec<Pop> {
    // Host pops of 2026-11-08-run34 game-host.log lines 1608-5457 (book, waves, kill count).
    let hits = |count: usize| std::iter::repeat_n((ATTACK_COUNT, None), count);
    hits(4)
        .chain([("EID_0200_a0030g_Talk02_End", None), (OWN_DAMAGE, None)])
        .chain(hits(8))
        .chain([("EID_0200_a0030g_Talk03_End", None)])
        .chain(hits(5))
        .chain([("EID_0200_a0030g_Talk04_End", None)])
        .chain(hits(1))
        .chain([
            ("EID_0200_a0032_End", None),
            ("EID_0200_a0035_01_01", None),
            ("EID_0200_a0037g_01_End", None),
            ("EID_0200_a0035_01_End", None),
            ("EID_0200_a0037_Lo_03", None),
            ("EID_0200_a0037g_02_Start", None),
            ("PUT_ENEMY_03", None),
            ("EID_0200_a0037g_EnemyCount", None),
            ("EID_0200_a0037g_EnemyCount_01", None),
            (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
            (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
            (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
            ("Talk_Text_02", None),
            ("RA_0200_a0040_01_radio", None),
        ])
        .collect()
}

fn run32_fight_pops() -> Vec<Pop> {
    // Host pops of 2026-11-08-run32 game-host.log lines 4306-11465, then the fight's end id.
    vec![
        (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
        (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
        (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
        ("Talk_Text_01_CA_END", None),
        ("CB_BossCA_01", Some(BATTLE_PHASE_MESSAGE)),
        ("EID_0200_a0060_Event", None),
        ("EID_0200_a0060_End", None),
        ("Talk_Text_03_SHOT", None),
        (OWN_DAMAGE, None),
        ("CB_BossCA_02", Some(BATTLE_PHASE_MESSAGE)),
        ("EID_0200_a0070_End", None),
        ("CB_BossCA_03", Some(BATTLE_PHASE_MESSAGE)),
        (FIGHT_END, None),
    ]
}

fn over_the_wire(message: Message) -> Message {
    assert_eq!(message.validate(), Ok(()));
    assert!(message.is_relayed());
    decode(&encode(&message).unwrap()).unwrap()
}

fn host_runs(pops: &[Pop]) -> HostRun {
    let mut host = HostFightStream::new();
    let mut outs = host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    let mut registered = 3;
    outs.extend(host.registered_callbacks(registered));
    outs.extend(host.popped(FIGHT_START, Vec::new(), false));
    assert!(matches!(outs.last(), Some(HostStreamOut::Armed(_))));
    outs.extend(host.registered_callbacks(registered));
    // The start handler registers the fight's callbacks; the next pop sees one more.
    registered += 1;
    outs.extend(host.registered_callbacks(registered));
    assert!(host.is_open_in(STONE_FIELD_PHASE));
    for (name, message) in pops {
        let args = message.map(StoryCallArg::Int).into_iter().collect();
        outs.extend(host.popped(name, args, false));
        // The host's handler ends before its next pop; the guest replays it only after this.
        outs.extend(host.script_slot(SlotState::Idle));
    }
    let closed_by = outs.iter().find_map(|out| match out {
        HostStreamOut::Closed { cause, .. } => Some(*cause),
        _ => None,
    });
    let streamed = outs
        .iter()
        .filter_map(|out| match out {
            HostStreamOut::Entry(entry) => Some((entry.name.clone(), entry.args.clone())),
            _ => None,
        })
        .collect();
    let messages = outs
        .iter()
        .filter_map(HostStreamOut::message)
        .map(over_the_wire)
        .collect();
    HostRun {
        messages,
        streamed,
        closed_by,
    }
}

#[derive(Default)]
struct GuestEngine {
    stream: GuestFightStream,
    partner_calls: EndQueue,
    own_fired: VecDeque<String>,
    queue: Vec<(String, bool)>,
    running_ticks: u32,
    stepping_held: bool,
    most_stream_entries_queued: usize,
    ran: Vec<String>,
    replayed: Vec<(String, Vec<StoryCallArg>)>,
    queued_args: Vec<Vec<StoryCallArg>>,
    held_own_pops: Vec<String>,
    ran_player_local: Vec<String>,
    ran_outside: Vec<String>,
}

impl GuestEngine {
    fn step(&mut self, now_ms: u64) {
        let phase = Some(STONE_FIELD_PHASE);
        if self.stepping_held {
            assert_eq!(
                self.stream
                    .tick(SlotState::SteppingHeld, phase, false, now_ms),
                None
            );
            return;
        }
        self.running_ticks = self.running_ticks.saturating_sub(1);
        if self.running_ticks == 0 {
            let popped = if let Some(name) = self.own_fired.pop_front() {
                Some((name, PopSource::Fired))
            } else if self.queue.is_empty() {
                None
            } else {
                Some((self.queue.remove(0).0, PopSource::Queued))
            };
            if let Some((name, source)) = popped {
                let registered_player_local = name == RADIO_HANDLER_OF_MESSAGE_210;
                match self
                    .stream
                    .pop(&name, source, phase, registered_player_local)
                {
                    GuestPop::RunReplayed { .. } => {
                        let args = self.queued_args.remove(0);
                        self.replayed.push((name.clone(), args));
                        self.ran.push(name);
                        self.running_ticks = 3;
                    }
                    GuestPop::RunPlayerLocal => {
                        self.ran_player_local.push(name.clone());
                        self.ran.push(name);
                        self.running_ticks = 1;
                    }
                    GuestPop::RunPartnerCall | GuestPop::Outside => {
                        self.ran_outside.push(name.clone());
                        self.ran.push(name);
                        self.running_ticks = 1;
                    }
                    GuestPop::Hold | GuestPop::Requested => self.held_own_pops.push(name),
                }
            }
        }
        let slot = if self.running_ticks > 0 {
            SlotState::Busy
        } else {
            SlotState::Idle
        };
        for event in self.partner_calls.advance(now_ms, |_| EndFollow::Mirror) {
            let name = match event {
                QueueEvent::Run { end, .. } => end.name,
                QueueEvent::RunCall { parked, .. } => parked.call.function,
                QueueEvent::Dropped { .. } | QueueEvent::StillWaiting { .. } => continue,
            };
            self.stream.partner_call_queued(&name, now_ms);
            self.queue.push((name, false));
        }
        let waiting = !self.partner_calls.is_empty();
        if let Some(entry) = self.stream.tick(slot, phase, waiting, now_ms) {
            self.queued_args.push(entry.args);
            self.queue.push((entry.name, true));
            let stream_entries = self.queue.iter().filter(|(_, ours)| *ours).count();
            self.most_stream_entries_queued = self.most_stream_entries_queued.max(stream_entries);
        }
    }
}

fn replay_on_guest(
    host: &HostRun,
    messages_per_tick: usize,
    own_fired: &[(usize, &str)],
    held_ticks: std::ops::Range<usize>,
    earlier_partner_call: bool,
) -> GuestEngine {
    let mut guest = GuestEngine::default();
    if earlier_partner_call {
        let call = StoryCall {
            phase: STONE_FIELD_PHASE,
            pack: 0,
            function: EARLIER_PARTNER_END.to_owned(),
            args: Vec::new(),
        };
        let parked = ParkedCall {
            kind: ParkedKind::Handler,
            call,
        };
        assert!(guest.partner_calls.park_call_always(parked, 0).is_some());
    }
    let mut arriving = host.messages.iter().cloned().peekable();
    for tick in 0..20_000 {
        let now_ms = tick as u64 * TICK_MS;
        for _ in 0..messages_per_tick {
            if let Some(message) = arriving.next() {
                let received = guest
                    .stream
                    .receive_message(message, Some(STONE_FIELD_PHASE));
                assert!(!matches!(
                    received,
                    GuestReceived::Entry {
                        arrival: GuestArrival::NotJoined,
                        ..
                    }
                ));
            }
        }
        for (at, name) in own_fired {
            if *at == tick {
                guest.own_fired.push_back((*name).to_owned());
            }
        }
        guest.stepping_held = held_ticks.contains(&tick);
        guest.step(now_ms);
        let done = arriving.peek().is_none()
            && !guest.stream.is_active_in(Some(STONE_FIELD_PHASE))
            && guest.queue.is_empty()
            && guest.own_fired.is_empty()
            && guest.running_ticks == 0;
        if tick > held_ticks.end && done {
            break;
        }
    }
    guest
}

#[test]
fn a_host_window_opens_on_the_first_registration_after_its_start_id_and_streams_from_seq_zero() {
    let host = host_runs(&run32_fight_pops());
    let Message::FightWindow(opened) = &host.messages[0] else {
        panic!("the first message opens the window");
    };
    assert_eq!((opened.number, opened.part), (1, 1));
    let seqs: Vec<u64> = host
        .messages
        .iter()
        .filter_map(|message| match message {
            Message::FightStream(entry) => {
                assert_eq!(entry.epoch, opened.epoch);
                Some(entry.seq)
            }
            _ => None,
        })
        .collect();
    assert_eq!(seqs, (0..seqs.len() as u64).collect::<Vec<_>>());
    assert_eq!(host.closed_by, Some(WindowCloseCause::End));
    assert!(!host.streamed.iter().any(|(name, _)| name == FIGHT_START));
    assert!(!host.streamed.iter().any(|(name, _)| name == OWN_DAMAGE));
    let fight = rc_game::fight_sync::fight_windows_of_pack(STONE_FIELD_PACK)
        .next()
        .unwrap();
    assert!(
        host.streamed
            .iter()
            .all(|(name, _)| !fight.is_player_local_handler(name))
    );
    assert!(BASE_PLAYER_LOCAL_HANDLERS.contains(&OWN_DAMAGE));
    assert_eq!(
        host.streamed.last().map(|(name, _)| name.as_str()),
        Some(FIGHT_END)
    );
}

#[test]
fn run34_fight_replays_on_the_guest_in_the_host_order_one_entry_queued_at_a_time() {
    let host = host_runs(&run34_fight_pops());
    assert_eq!(host.closed_by, None);
    for messages_per_tick in [1, 4, 1000] {
        let guest = replay_on_guest(&host, messages_per_tick, &[], 0..0, false);
        assert_eq!(guest.replayed, host.streamed);
        assert_eq!(guest.most_stream_entries_queued, 1);
        assert!(guest.held_own_pops.is_empty());
    }
}

#[test]
fn run34_guest_own_waves_and_hits_are_held_and_its_player_local_pops_run() {
    let host = host_runs(&run34_fight_pops());
    let own = [
        (2, ATTACK_COUNT),
        (9, "EID_0200_a0030g_Talk02_End"),
        (30, OWN_DAMAGE),
        (31, "PUT_ENEMY_03"),
        (40, RADIO_HANDLER_OF_MESSAGE_210),
        (55, "EID_0200_a0037g_EnemyCount"),
    ];
    let guest = replay_on_guest(&host, 1, &own, 0..0, false);
    assert_eq!(guest.replayed, host.streamed);
    assert_eq!(
        guest.held_own_pops,
        [
            ATTACK_COUNT,
            "EID_0200_a0030g_Talk02_End",
            "PUT_ENEMY_03",
            "EID_0200_a0037g_EnemyCount"
        ]
    );
    assert_eq!(
        guest.ran_player_local,
        [OWN_DAMAGE, RADIO_HANDLER_OF_MESSAGE_210]
    );
    assert_eq!(guest.most_stream_entries_queued, 1);
}

#[test]
fn run32_fight_replays_battle_phases_with_their_message_arguments_and_then_leaves_the_window() {
    let host = host_runs(&run32_fight_pops());
    let guest = replay_on_guest(&host, 2, &[(4, "CB_BossCA_01")], 0..0, false);
    assert_eq!(guest.replayed, host.streamed);
    let battle: Vec<&str> = guest
        .replayed
        .iter()
        .filter(|(_, args)| *args == [StoryCallArg::Int(BATTLE_PHASE_MESSAGE)])
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(battle, ["CB_BossCA_01", "CB_BossCA_02", "CB_BossCA_03"]);
    assert_eq!(guest.held_own_pops, ["CB_BossCA_01"]);
    assert!(!guest.stream.is_active_in(Some(STONE_FIELD_PHASE)));
}

#[test]
fn a_partner_call_queued_before_the_window_runs_before_the_first_host_entry() {
    let host = host_runs(&run32_fight_pops());
    let guest = replay_on_guest(&host, 1000, &[], 0..0, true);
    assert_eq!(
        guest.ran.first().map(String::as_str),
        Some(EARLIER_PARTNER_END)
    );
    assert_eq!(guest.ran_outside, [EARLIER_PARTNER_END]);
    assert_eq!(guest.replayed, host.streamed);
    assert_eq!(guest.most_stream_entries_queued, 1);
}

#[test]
fn a_guest_menu_stretch_holds_the_replay_and_drops_nothing() {
    let host = host_runs(&run32_fight_pops());
    let guest = replay_on_guest(&host, 1, &[], 3..80, false);
    assert_eq!(guest.replayed, host.streamed);
    assert_eq!(guest.most_stream_entries_queued, 1);
}

#[test]
fn a_host_restart_inside_the_window_keeps_it_open_at_a_new_epoch_and_counts_once() {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    host.popped(FIGHT_START, Vec::new(), false);
    host.registered_callbacks(2);
    let opened = host.registered_callbacks(3);
    let first_epoch = match opened.as_slice() {
        [HostStreamOut::Opened { epoch, .. }] => *epoch,
        other => panic!("expected an opening, got {other:?}"),
    };
    host.popped(ATTACK_COUNT, Vec::new(), false);
    let restarted = host.restart();
    let [HostStreamOut::Restarted { epoch, .. }] = restarted.as_slice() else {
        panic!("expected a restart mark, got {restarted:?}");
    };
    let epoch = *epoch;
    assert!(epoch > first_epoch);
    assert!(host.is_open_in(STONE_FIELD_PHASE));
    assert_eq!(host.restart(), Vec::new());
    let HostStreamOut::Entry(entry) = &host.popped(ATTACK_COUNT, Vec::new(), false)[0] else {
        panic!("expected an entry");
    };
    assert_eq!((entry.epoch, entry.seq), (epoch, 0));
}

#[test]
fn a_game_over_or_continue_id_inside_the_window_is_part_of_the_fight() {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    host.popped(FIGHT_START, Vec::new(), false);
    host.registered_callbacks(0);
    host.registered_callbacks(1);
    let fight = rc_game::fight_sync::fight_windows_of_pack(STONE_FIELD_PACK)
        .next()
        .unwrap();
    for name in fight.game_over_ids.iter().chain(fight.continue_ids) {
        host.popped(name, Vec::new(), false);
        assert!(
            host.is_open_in(STONE_FIELD_PHASE),
            "{name} closed the window"
        );
    }
    let closed = host.popped(FIGHT_END, Vec::new(), false);
    assert!(matches!(
        closed.last(),
        Some(HostStreamOut::Closed {
            cause: WindowCloseCause::End,
            ..
        })
    ));
}

#[test]
fn a_host_restart_while_only_armed_closes_the_window() {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    host.popped(FIGHT_START, Vec::new(), false);
    assert!(host.is_armed());
    assert!(matches!(
        host.restart().as_slice(),
        [HostStreamOut::Closed {
            cause: WindowCloseCause::Restart,
            end_seq: 0,
            ..
        }]
    ));
}

#[test]
fn a_guest_drops_entries_of_an_epoch_before_the_host_restart() {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    host.popped(FIGHT_START, Vec::new(), false);
    host.registered_callbacks(0);
    let first = host.registered_callbacks(1);
    let old = host.popped(ATTACK_COUNT, Vec::new(), false);
    let restarted = host.restart();
    let mut guest = GuestFightStream::new();
    let here = Some(STONE_FIELD_PHASE);
    let message = |outs: &[HostStreamOut]| outs[0].message().unwrap();
    guest.receive_message(message(&first), here);
    guest.receive_message(message(&restarted), here);
    let received = guest.receive_message(message(&old), here);
    assert!(matches!(
        received,
        GuestReceived::Entry {
            arrival: GuestArrival::Stream(rc_game::fight_sync::Arrival::Stale { .. }),
            ..
        }
    ));
}

#[test]
fn a_guest_in_another_phase_does_not_join_and_keeps_its_own_pops() {
    let host = host_runs(&run32_fight_pops());
    let mut guest = GuestFightStream::new();
    let elsewhere = Some(STONE_FIELD_PHASE + 1);
    let opened = guest.receive_message(host.messages[0].clone(), elsewhere);
    assert!(matches!(
        opened,
        GuestReceived::Opened {
            open: GuestOpen::NotHere,
            ..
        }
    ));
    let entry = guest.receive_message(host.messages[1].clone(), Some(STONE_FIELD_PHASE));
    assert!(matches!(
        entry,
        GuestReceived::Entry {
            arrival: GuestArrival::NotJoined,
            ..
        }
    ));
    let here = Some(STONE_FIELD_PHASE);
    assert_eq!(
        guest.pop(KILL_COUNT_END, PopSource::Fired, here, false),
        GuestPop::Outside
    );
    assert_eq!(guest.tick(SlotState::Idle, here, false, 0), None);
}

#[test]
fn a_queued_entry_cleared_from_the_engine_queue_is_queued_again() {
    let host = host_runs(&run32_fight_pops());
    let mut guest = GuestFightStream::new();
    let here = Some(STONE_FIELD_PHASE);
    for message in host.messages.iter().take(3).cloned() {
        guest.receive_message(message, here);
    }
    let first = guest.tick(SlotState::Idle, here, false, 0).unwrap();
    assert_eq!(guest.tick(SlotState::Idle, here, false, 0), None);
    assert_eq!(guest.queued_call_lost(), Some(first.seq));
    assert_eq!(guest.tick(SlotState::Idle, here, false, 0), Some(first));
}

#[test]
fn a_partner_call_that_never_pops_stops_holding_the_stream_after_its_time() {
    let host = host_runs(&run32_fight_pops());
    let mut guest = GuestFightStream::new();
    let here = Some(STONE_FIELD_PHASE);
    for message in host.messages.iter().take(3).cloned() {
        guest.receive_message(message, here);
    }
    guest.partner_call_queued(EARLIER_PARTNER_END, 0);
    assert_eq!(guest.tick(SlotState::Idle, here, false, 10), None);
    let later = PARTNER_CALL_POP_EXPECTED_MS + 1;
    assert!(guest.tick(SlotState::Idle, here, false, later).is_some());
    assert_eq!(guest.partner_calls_queued(), 0);
    assert!(matches!(guest.in_flight(), InFlight::Queued { seq: 0 }));
}

#[test]
fn a_pop_is_told_fired_or_queued_by_the_argument_block_it_points_at() {
    let fired_first = MANAGER + 0x70;
    assert_eq!(pop_source(MANAGER, fired_first), Some(PopSource::Fired));
    assert_eq!(
        pop_source(MANAGER, fired_first + 255 * CALLBACK_REGISTRY_ENTRY_STRIDE),
        Some(PopSource::Fired)
    );
    let queued_first = MANAGER + QUEUE_FIRST_SLOT_OFFSET + QUEUED_CALL_ARGUMENTS_OFFSET;
    assert_eq!(queued_first, MANAGER + 0x8_0C30);
    assert_eq!(pop_source(MANAGER, queued_first), Some(PopSource::Queued));
    assert_eq!(
        pop_source(MANAGER, queued_first + 63 * QUEUE_SLOT_STRIDE),
        Some(PopSource::Queued)
    );
    assert_eq!(
        pop_source(MANAGER, queued_first + 64 * QUEUE_SLOT_STRIDE),
        None
    );
    assert_eq!(pop_source(MANAGER, fired_first + 8), None);
    assert_eq!(pop_source(MANAGER, 0), None);
    assert_eq!(
        callback_registry_kind_address(MANAGER, 1),
        MANAGER + 4 + 0x7C8
    );
}

#[test]
fn the_script_slot_is_idle_only_in_the_states_that_pop_next() {
    assert_eq!(script_slot_state(Some(0x6C8)), SlotState::Idle);
    assert_eq!(script_slot_state(Some(0x6CD)), SlotState::Idle);
    assert_eq!(script_slot_state(Some(0x6C3)), SlotState::Busy);
    assert_eq!(script_slot_state(Some(0x6D1)), SlotState::Busy);
    assert_eq!(script_slot_state(Some(0)), SlotState::Busy);
    assert_eq!(script_slot_state(None), SlotState::Busy);
    assert_eq!(
        script_slot_state(Some(SCRIPT_SLOT_STEPPING_HELD_STATE)),
        SlotState::SteppingHeld
    );
}
