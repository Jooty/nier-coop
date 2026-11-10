use rc_game::fight_sync::{
    GuestArrival, GuestFightStream, GuestNotice, GuestPop, GuestRadioWaitCall, GuestReceived,
    HOST_ENTRY_END_WAIT_LIMIT_MS, HOST_ENTRY_END_WAIT_LOGGED_FROM_MS, HostFightStream,
    HostStreamOut, InFlight, PopSource, RadioWaitRelease, STREAM_STALL_LOGGED_FROM_MS, SlotState,
    TapeValue,
};
use rc_proto::{FightRadioWaitEnded, FightWindowChange, FightWindowMark, Message, decode, encode};

const STONE_FIELD_PACK: &str = "A_STONE_FIELD_02";
const STONE_FIELD_PHASE: u16 = 23;
const FIGHT_ARM_POP: &str = "EID_0200_a0030g_Talk01";
const FIGHT_END: &str = "EID_0200_a0080_End";
const WAITING_ENTRY: &str = "REPEAT_QUESTION";
const BOSS_PHASE_TWO: &str = "CB_BossCA_02";
const UNTAPED_WINDOW_FUNCTION: &str = "EID_0200_a0030g_End";
const NAME_IN_NO_WINDOW_LIST: &str = "ALL_MAGIC_LEARN";
const SCENE_HANDLER_AFTER_THE_RADIO: &str = "EID_0200_a0040g_Talk";
const GUEST_ONLY_GIMMICK: &str = "EID_0200_a0034_Action";
const HERE: Option<u16> = Some(STONE_FIELD_PHASE);

fn over_the_wire(message: Message) -> Message {
    assert_eq!(message.validate(), Ok(()));
    assert!(message.is_relayed());
    decode(&encode(&message).unwrap()).unwrap()
}

fn messages(outs: &[HostStreamOut]) -> Vec<Message> {
    outs.iter()
        .filter_map(HostStreamOut::message)
        .map(over_the_wire)
        .collect()
}

fn open_fight() -> (HostFightStream, GuestFightStream) {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    let opened = host.popped(FIGHT_ARM_POP, Vec::new(), false);
    // The guest's own copy of the arming pop ran first, so it skips the host's entry 0.
    let mut guest = GuestFightStream::new();
    guest.pop(FIGHT_ARM_POP, PopSource::Fired, 0, HERE, false);
    let received: Vec<GuestReceived> = messages(&opened)
        .into_iter()
        .map(|message| guest.receive_message(message, HERE))
        .collect();
    assert!(matches!(
        received.as_slice(),
        [
            GuestReceived::Opened { .. },
            GuestReceived::Entry {
                arrival: GuestArrival::ArmingEntrySkipped,
                ..
            }
        ]
    ));
    for message in host_slot_idle(&mut host) {
        guest.receive_message(message, HERE);
    }
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::ArmingEntrySkipped { .. }]
    ));
    (host, guest)
}

fn host_reads(host: &mut HostFightStream, native: &str, value: TapeValue) -> Message {
    over_the_wire(
        host.taped_read(native, value)
            .expect("a read inside a streamed handler is taped")
            .message,
    )
}

fn host_slot_idle(host: &mut HostFightStream) -> Vec<Message> {
    let ended: Vec<HostStreamOut> = host.script_slot(SlotState::Idle).into_iter().collect();
    messages(&ended)
}

fn entry_ended_seqs(messages: &[Message]) -> Vec<u64> {
    messages
        .iter()
        .filter_map(|message| match message {
            Message::FightWindow(FightWindowMark {
                change: FightWindowChange::EntryEnded { seq },
                ..
            }) => Some(*seq),
            _ => None,
        })
        .collect()
}

#[test]
fn the_host_marks_each_streamed_entry_ended_once_after_every_tape_value_of_it() {
    let (mut host, _) = open_fight();
    let mut sent = messages(&host.popped(WAITING_ENTRY, Vec::new(), false));
    sent.push(host_reads(&mut host, "_GetActorLife", TapeValue::Int(90)));
    assert!(host.script_slot(SlotState::Busy).is_none());
    assert!(host.script_slot(SlotState::SteppingHeld).is_none());
    sent.extend(host_slot_idle(&mut host));
    sent.extend(host_slot_idle(&mut host));
    // The next pop also ends a handler that never went idle on a tick.
    sent.extend(messages(&host.popped(BOSS_PHASE_TWO, Vec::new(), false)));
    sent.push(host_reads(&mut host, "_GetRandI", TapeValue::Int(3)));
    sent.extend(messages(&host.popped("LIB_GameOver", Vec::new(), false)));
    let kinds: Vec<String> = sent
        .iter()
        .map(|message| match message {
            Message::FightStream(entry) => format!("entry {}", entry.seq),
            Message::FightTape(tape) => format!("tape {}", tape.seq),
            Message::FightWindow(FightWindowMark {
                change: FightWindowChange::EntryEnded { seq },
                ..
            }) => format!("ended {seq}"),
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "entry 1", "tape 1", "ended 1", "entry 2", "tape 2", "ended 2"
        ]
    );
}

#[test]
fn a_guest_entry_waits_for_the_hosts_end_of_it_then_takes_a_value_read_after_it_arrived() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(WAITING_ENTRY, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 100), None);
    assert_eq!(guest.waiting_for_host_entry_end(), Some(1));
    assert_eq!(guest.in_flight(), InFlight::Idle);
    // The host reads late in its handler, after the entry reached the guest.
    let late = host_reads(&mut host, "_GetActorLifeRate", TapeValue::Float(0.25));
    guest.receive_message(late, HERE);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 200), None);
    for message in host_slot_idle(&mut host) {
        assert_eq!(
            guest.receive_message(message, HERE),
            GuestReceived::EntryEnded { seq: 1, kept: true }
        );
    }
    let queued = guest
        .tick(SlotState::Idle, HERE, false, 300)
        .expect("released once the host's handler ended");
    assert_eq!(queued.seq, 1);
    assert_eq!(guest.waiting_for_host_entry_end(), None);
    assert_eq!(
        guest.pop(&queued.name, PopSource::Queued, 0, HERE, false),
        GuestPop::RunReplayed { seq: 1 }
    );
    let read = guest
        .read_taped(
            "_GetActorLifeRate",
            TapeValue::Float(0.5),
            HERE,
            SlotState::Busy,
        )
        .unwrap();
    assert_eq!(read.value, TapeValue::Float(0.25));
    assert!(
        guest.take_notices().is_empty(),
        "a 200 ms wait is not logged"
    );
}

#[test]
fn a_long_wait_for_the_hosts_end_is_logged_with_its_length() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(WAITING_ENTRY, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 1_000), None);
    // The guest's menu holds its stepping meanwhile; the wait goes on and drops nothing.
    assert_eq!(
        guest.tick(SlotState::SteppingHeld, HERE, false, 2_000),
        None
    );
    for message in host_slot_idle(&mut host) {
        guest.receive_message(message, HERE);
    }
    let waited_ms = HOST_ENTRY_END_WAIT_LOGGED_FROM_MS + 500;
    let queued = guest.tick(SlotState::Idle, HERE, false, 1_000 + waited_ms);
    assert_eq!(queued.map(|entry| entry.seq), Some(1));
    assert_eq!(
        guest.take_notices(),
        [GuestNotice::WaitedForHostEntryEnd { seq: 1, waited_ms }]
    );
}

#[test]
fn an_end_that_never_comes_releases_the_entry_after_the_limit_with_one_line() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(WAITING_ENTRY, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 0), None);
    assert_eq!(
        guest.tick(SlotState::Idle, HERE, false, HOST_ENTRY_END_WAIT_LIMIT_MS),
        None
    );
    let late = HOST_ENTRY_END_WAIT_LIMIT_MS + 1;
    let queued = guest.tick(SlotState::Idle, HERE, false, late);
    assert_eq!(queued.map(|entry| entry.seq), Some(1));
    assert_eq!(
        guest.take_notices(),
        [GuestNotice::HostEntryEndNeverCame {
            seq: 1,
            waited_ms: late
        }]
    );
}

#[test]
fn an_end_mark_of_an_old_epoch_or_another_phase_releases_nothing() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(WAITING_ENTRY, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    let old_end = host_slot_idle(&mut host);
    let restarted = host.restart();
    for message in messages(&restarted) {
        guest.receive_message(message, HERE);
    }
    guest.own_restart(50);
    for message in messages(&host.popped(WAITING_ENTRY, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    for message in old_end {
        assert_eq!(
            guest.receive_message(message, HERE),
            GuestReceived::EntryEnded {
                seq: 1,
                kept: false
            }
        );
    }
    let new_end = host_slot_idle(&mut host);
    let Message::FightWindow(mark) = &new_end[0] else {
        panic!("an end mark");
    };
    let elsewhere = Message::FightWindow(FightWindowMark {
        phase: STONE_FIELD_PHASE + 1,
        ..mark.clone()
    });
    assert_eq!(
        guest.receive_message(elsewhere, HERE),
        GuestReceived::EntryEnded {
            seq: 0,
            kept: false
        }
    );
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 100), None);
    guest.receive_message(new_end[0].clone(), HERE);
    let queued = guest.tick(SlotState::Idle, HERE, false, 200).unwrap();
    assert_eq!((queued.epoch, queued.seq), (host.epoch(), 0));
}

#[test]
fn the_end_ids_entry_is_marked_ended_before_the_close_so_the_guest_drains_and_leaves() {
    let (mut host, mut guest) = open_fight();
    let mut sent = messages(&host.popped(WAITING_ENTRY, Vec::new(), false));
    sent.extend(messages(&host.popped(FIGHT_END, Vec::new(), false)));
    assert_eq!(entry_ended_seqs(&sent), [1, 2]);
    assert!(matches!(
        sent.last(),
        Some(Message::FightWindow(FightWindowMark {
            change: FightWindowChange::Closed { end_seq: 3 },
            ..
        }))
    ));
    assert!(host_slot_idle(&mut host).is_empty(), "nothing left to end");
    for message in sent {
        guest.receive_message(message, HERE);
    }
    let mut ran = Vec::new();
    let mut now_ms = 0;
    while let Some(entry) = guest.tick(SlotState::Idle, HERE, false, now_ms) {
        guest.pop(&entry.name, PopSource::Queued, 0, HERE, false);
        ran.push(entry.name);
        now_ms += 33;
    }
    assert_eq!(ran, [WAITING_ENTRY, FIGHT_END]);
    assert!(!guest.is_active_in(HERE));
}

#[test]
fn a_held_own_pop_the_hosts_stream_never_pairs_with_is_logged_once_per_name_when_the_epoch_ends() {
    let (mut host, mut guest) = open_fight();
    guest.tick(SlotState::Idle, HERE, false, 0);
    assert_eq!(
        guest.pop(GUEST_ONLY_GIMMICK, PopSource::Queued, 0, HERE, false),
        GuestPop::Hold
    );
    assert_eq!(
        guest.pop(GUEST_ONLY_GIMMICK, PopSource::Queued, 0, HERE, false),
        GuestPop::Hold
    );
    // A held pop the host's stream then carries is matched, not logged.
    assert_eq!(
        guest.pop(WAITING_ENTRY, PopSource::Fired, 0, HERE, false),
        GuestPop::Hold
    );
    for message in messages(&host.popped(WAITING_ENTRY, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    // No clock: a host copy may still come any time in the epoch.
    guest.tick(SlotState::Busy, HERE, false, 600_000);
    assert!(guest.take_notices().is_empty());
    guest.leave();
    assert_eq!(
        guest.take_notices(),
        [GuestNotice::HeldPopNeverMatched {
            name: GUEST_ONLY_GIMMICK.to_owned(),
            epoch: host.epoch()
        }]
    );
}

#[test]
fn a_held_own_pop_still_unmatched_when_the_guest_leaves_the_fight_is_logged() {
    let (_, mut guest) = open_fight();
    guest.tick(SlotState::Idle, HERE, false, 0);
    guest.pop(GUEST_ONLY_GIMMICK, PopSource::Queued, 0, HERE, false);
    guest.leave();
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::HeldPopNeverMatched { name, .. }] if name == GUEST_ONLY_GIMMICK
    ));
}

fn guest_receives_host_pop(host: &mut HostFightStream, guest: &mut GuestFightStream, name: &str) {
    for message in messages(&host.popped(name, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
}

#[test]
fn an_entry_that_reads_no_taped_native_starts_on_the_guest_without_the_hosts_end_mark() {
    let (mut host, mut guest) = open_fight();
    guest_receives_host_pop(&mut host, &mut guest, UNTAPED_WINDOW_FUNCTION);
    let queued = guest
        .tick(SlotState::Idle, HERE, false, 100)
        .expect("an untaped entry is released at once");
    assert_eq!(queued.name, UNTAPED_WINDOW_FUNCTION);
    assert_eq!(guest.waiting_for_host_entry_end(), None);
    assert_eq!(
        guest.pop(&queued.name, PopSource::Queued, 0, HERE, false),
        GuestPop::RunReplayed { seq: 1 }
    );
}

#[test]
fn an_untaped_entry_still_keeps_one_entry_in_flight() {
    let (mut host, mut guest) = open_fight();
    guest_receives_host_pop(&mut host, &mut guest, UNTAPED_WINDOW_FUNCTION);
    guest_receives_host_pop(&mut host, &mut guest, UNTAPED_WINDOW_FUNCTION);
    let first = guest.tick(SlotState::Idle, HERE, false, 100).unwrap();
    assert_eq!(first.seq, 1);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 110), None);
    assert_eq!(
        guest.pop(&first.name, PopSource::Queued, 0, HERE, false),
        GuestPop::RunReplayed { seq: 1 }
    );
    assert_eq!(guest.tick(SlotState::Busy, HERE, false, 120), None);
    let second = guest.tick(SlotState::Idle, HERE, false, 130).unwrap();
    assert_eq!(second.seq, 2);
}

#[test]
fn run36_the_scene_handler_a_radio_registers_is_a_window_function_and_starts_at_once() {
    // Run36: it was in no window list, so the guest waited 10 s for the host's end of it while
    // the host's copy sat 60 s in its radio line, and the scene never came on the guest.
    let (mut host, mut guest) = open_fight();
    guest_receives_host_pop(&mut host, &mut guest, SCENE_HANDLER_AFTER_THE_RADIO);
    let queued = guest
        .tick(SlotState::Idle, HERE, false, 100)
        .expect("no taped native in its call tree, so no wait for the host's end");
    assert_eq!(queued.name, SCENE_HANDLER_AFTER_THE_RADIO);
    assert_eq!(guest.waiting_for_host_entry_end(), None);
    assert!(guest.take_notices().is_empty());
}

#[test]
fn an_entry_named_in_no_window_list_waits_for_the_hosts_end_mark() {
    let (mut host, mut guest) = open_fight();
    guest_receives_host_pop(&mut host, &mut guest, NAME_IN_NO_WINDOW_LIST);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 100), None);
    assert_eq!(guest.waiting_for_host_entry_end(), Some(1));
    for message in host_slot_idle(&mut host) {
        guest.receive_message(message, HERE);
    }
    let queued = guest.tick(SlotState::Idle, HERE, false, 200).unwrap();
    assert_eq!(queued.name, NAME_IN_NO_WINDOW_LIST);
}

fn radio_call(starts: bool, now_ms: u64) -> GuestRadioWaitCall {
    GuestRadioWaitCall {
        starts,
        still_waiting: true,
        host_in_scene: false,
        here_in_scene: false,
        line_advancing: false,
        now_ms,
    }
}

#[test]
fn the_guests_copy_of_a_host_radio_wait_lasts_as_long_as_the_hosts_even_after_the_host_ended() {
    let (mut host, mut guest) = open_fight();
    let mut sent = messages(&host.popped(WAITING_ENTRY, Vec::new(), false));
    assert_eq!(host.radio_wait_polled(true, true, 1_000), None);
    assert_eq!(host.radio_wait_polled(false, true, 2_000), None);
    let ended = host
        .radio_wait_polled(false, false, 4_000)
        .map(over_the_wire)
        .expect("a wait of a streamed entry is marked when it ends");
    assert!(matches!(
        ended,
        Message::FightRadioWait(FightRadioWaitEnded {
            phase: STONE_FIELD_PHASE,
            seq: 1,
            wait_index: 0,
            host_waited_ms: 3_000,
            ..
        })
    ));
    sent.push(ended);
    sent.extend(host_slot_idle(&mut host));
    let received: Vec<GuestReceived> = sent
        .into_iter()
        .map(|message| guest.receive_message(message, HERE))
        .collect();
    assert!(received.contains(&GuestReceived::RadioWaitEnded {
        seq: 1,
        wait_index: 0,
        kept: true
    }));
    let queued = guest.tick(SlotState::Idle, HERE, false, 5_000).unwrap();
    assert_eq!(
        guest.pop(&queued.name, PopSource::Queued, 0, HERE, false),
        GuestPop::RunReplayed { seq: 1 }
    );
    // The host's handler already ended: its mark, not its end, says how long this one waits.
    assert_eq!(guest.radio_wait_polled(radio_call(true, 5_100), HERE), None);
    assert_eq!(
        guest.radio_wait_polled(radio_call(false, 8_099), HERE),
        None
    );
    assert_eq!(
        guest.radio_wait_polled(radio_call(false, 8_100), HERE),
        Some(RadioWaitRelease::Mark {
            line_advancing: false
        })
    );
}

#[test]
fn a_host_radio_wait_outside_a_streamed_handler_sends_nothing() {
    let (mut host, _) = open_fight();
    assert_eq!(host.radio_wait_polled(true, true, 0), None);
    assert_eq!(host.radio_wait_polled(false, false, 3_000), None);
}

#[test]
fn a_guest_radio_wait_outside_the_joined_fight_is_left_to_the_game() {
    let mut guest = GuestFightStream::new();
    let host_in_scene = GuestRadioWaitCall {
        host_in_scene: true,
        ..radio_call(true, 0)
    };
    assert_eq!(guest.radio_wait_polled(host_in_scene, HERE), None);
}

#[test]
fn a_host_entry_stuck_behind_the_guests_slot_is_named_after_five_seconds() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(UNTAPED_WINDOW_FUNCTION, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    let queued = guest.tick(SlotState::Idle, HERE, false, 0).unwrap();
    guest.pop(&queued.name, PopSource::Queued, 0, HERE, false);
    for message in messages(&host.popped(BOSS_PHASE_TWO, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    assert_eq!(guest.tick(SlotState::Busy, HERE, false, 1_000), None);
    assert_eq!(guest.next_waiting_entry(), Some((2, BOSS_PHASE_TWO)));
    assert_eq!(
        guest.stream_stall(1_000 + STREAM_STALL_LOGGED_FROM_MS),
        None
    );
    assert_eq!(guest.tick(SlotState::Busy, HERE, false, 6_001), None);
    assert_eq!(
        guest.stream_stall(1_001 + STREAM_STALL_LOGGED_FROM_MS),
        Some((
            2,
            BOSS_PHASE_TWO.to_owned(),
            STREAM_STALL_LOGGED_FROM_MS + 1
        ))
    );
    // The host had no radio wait there and has moved on, so the guest's one ends at once.
    assert_eq!(
        guest.radio_wait_polled(radio_call(true, 6_002), HERE),
        Some(RadioWaitRelease::HostMovedOn)
    );
}
