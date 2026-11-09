use rc_game::fight_sync::{
    GuestFightStream, GuestNotice, GuestPop, GuestReceived, HOST_ENTRY_END_WAIT_LIMIT_MS,
    HOST_ENTRY_END_WAIT_LOGGED_FROM_MS, HostFightStream, HostStreamOut, InFlight, PopSource,
    SAME_POP_ON_BOTH_GAMES_SPAN_MS, SlotState, TapeValue,
};
use rc_proto::{FightWindowChange, FightWindowMark, Message, decode, encode};

const STONE_FIELD_PACK: &str = "A_STONE_FIELD_02";
const STONE_FIELD_PHASE: u16 = 23;
const FIGHT_ARM_POP: &str = "EID_0200_a0030g_Talk01";
const FIGHT_END: &str = "EID_0200_a0080_End";
const BOSS_PHASE_ONE: &str = "CB_BossCA_01";
const BOSS_PHASE_TWO: &str = "CB_BossCA_02";
const UNTAPED_WINDOW_FUNCTION: &str = "EID_0200_a0030g_End";
const NAME_IN_NO_WINDOW_LIST: &str = "EID_0200_a0037g_EnemyCount";
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
    let mut guest = GuestFightStream::new();
    for message in messages(&opened) {
        let received = guest.receive_message(message, HERE);
        assert!(matches!(received, GuestReceived::Opened { .. }));
    }
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
    let mut sent = messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false));
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
            "entry 0", "tape 0", "ended 0", "entry 1", "tape 1", "ended 1"
        ]
    );
}

#[test]
fn a_guest_entry_waits_for_the_hosts_end_of_it_then_takes_a_value_read_after_it_arrived() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 100), None);
    assert_eq!(guest.waiting_for_host_entry_end(), Some(0));
    assert_eq!(guest.in_flight(), InFlight::Idle);
    // The host reads late in its handler, after the entry reached the guest.
    let late = host_reads(&mut host, "_GetActorLifeRate", TapeValue::Float(0.25));
    guest.receive_message(late, HERE);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 200), None);
    for message in host_slot_idle(&mut host) {
        assert_eq!(
            guest.receive_message(message, HERE),
            GuestReceived::EntryEnded { seq: 0, kept: true }
        );
    }
    let queued = guest
        .tick(SlotState::Idle, HERE, false, 300)
        .expect("released once the host's handler ended");
    assert_eq!(queued.seq, 0);
    assert_eq!(guest.waiting_for_host_entry_end(), None);
    assert_eq!(
        guest.pop(&queued.name, PopSource::Queued, 0, HERE, false),
        GuestPop::RunReplayed { seq: 0 }
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
    for message in messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false)) {
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
    assert_eq!(queued.map(|entry| entry.seq), Some(0));
    assert_eq!(
        guest.take_notices(),
        [GuestNotice::WaitedForHostEntryEnd { seq: 0, waited_ms }]
    );
}

#[test]
fn an_end_that_never_comes_releases_the_entry_after_the_limit_with_one_line() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 0), None);
    assert_eq!(
        guest.tick(SlotState::Idle, HERE, false, HOST_ENTRY_END_WAIT_LIMIT_MS),
        None
    );
    let late = HOST_ENTRY_END_WAIT_LIMIT_MS + 1;
    let queued = guest.tick(SlotState::Idle, HERE, false, late);
    assert_eq!(queued.map(|entry| entry.seq), Some(0));
    assert_eq!(
        guest.take_notices(),
        [GuestNotice::HostEntryEndNeverCame {
            seq: 0,
            waited_ms: late
        }]
    );
}

#[test]
fn an_end_mark_of_an_old_epoch_or_another_phase_releases_nothing() {
    let (mut host, mut guest) = open_fight();
    for message in messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    let old_end = host_slot_idle(&mut host);
    let restarted = host.restart();
    for message in messages(&restarted) {
        guest.receive_message(message, HERE);
    }
    guest.own_restart(50);
    for message in messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    for message in old_end {
        assert_eq!(
            guest.receive_message(message, HERE),
            GuestReceived::EntryEnded {
                seq: 0,
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
        ..*mark
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
    let mut sent = messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false));
    sent.extend(messages(&host.popped(FIGHT_END, Vec::new(), false)));
    assert_eq!(entry_ended_seqs(&sent), [0, 1]);
    assert!(matches!(
        sent.last(),
        Some(Message::FightWindow(FightWindowMark {
            change: FightWindowChange::Closed { end_seq: 2 },
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
    assert_eq!(ran, [BOSS_PHASE_ONE, FIGHT_END]);
    assert!(!guest.is_active_in(HERE));
}

#[test]
fn a_held_own_pop_the_hosts_stream_never_runs_is_logged_once_per_name_and_epoch() {
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
        guest.pop(BOSS_PHASE_ONE, PopSource::Fired, 0, HERE, false),
        GuestPop::Hold
    );
    for message in messages(&host.popped(BOSS_PHASE_ONE, Vec::new(), false)) {
        guest.receive_message(message, HERE);
    }
    guest.tick(SlotState::Busy, HERE, false, SAME_POP_ON_BOTH_GAMES_SPAN_MS);
    assert!(guest.take_notices().is_empty());
    let past = SAME_POP_ON_BOTH_GAMES_SPAN_MS + 1;
    guest.tick(SlotState::Busy, HERE, false, past);
    assert_eq!(
        guest.take_notices(),
        [GuestNotice::HeldPopNeverMatched {
            name: GUEST_ONLY_GIMMICK.to_owned(),
            epoch: host.epoch()
        }]
    );
    guest.pop(GUEST_ONLY_GIMMICK, PopSource::Queued, 0, HERE, false);
    guest.leave();
    assert!(
        guest.take_notices().is_empty(),
        "already logged in this epoch"
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
        GuestPop::RunReplayed { seq: 0 }
    );
}

#[test]
fn an_untaped_entry_still_keeps_one_entry_in_flight() {
    let (mut host, mut guest) = open_fight();
    guest_receives_host_pop(&mut host, &mut guest, UNTAPED_WINDOW_FUNCTION);
    guest_receives_host_pop(&mut host, &mut guest, UNTAPED_WINDOW_FUNCTION);
    let first = guest.tick(SlotState::Idle, HERE, false, 100).unwrap();
    assert_eq!(first.seq, 0);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 110), None);
    assert_eq!(
        guest.pop(&first.name, PopSource::Queued, 0, HERE, false),
        GuestPop::RunReplayed { seq: 0 }
    );
    assert_eq!(guest.tick(SlotState::Busy, HERE, false, 120), None);
    let second = guest.tick(SlotState::Idle, HERE, false, 130).unwrap();
    assert_eq!(second.seq, 1);
}

#[test]
fn an_entry_whose_call_tree_reads_a_taped_native_waits_for_the_hosts_end_mark() {
    let (mut host, mut guest) = open_fight();
    guest_receives_host_pop(&mut host, &mut guest, BOSS_PHASE_ONE);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 100), None);
    assert_eq!(guest.waiting_for_host_entry_end(), Some(0));
    for message in host_slot_idle(&mut host) {
        guest.receive_message(message, HERE);
    }
    let queued = guest.tick(SlotState::Idle, HERE, false, 200).unwrap();
    assert_eq!(queued.name, BOSS_PHASE_ONE);
}

#[test]
fn an_entry_named_in_no_window_list_waits_for_the_hosts_end_mark() {
    let (mut host, mut guest) = open_fight();
    guest_receives_host_pop(&mut host, &mut guest, NAME_IN_NO_WINDOW_LIST);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 100), None);
    assert_eq!(guest.waiting_for_host_entry_end(), Some(0));
    for message in host_slot_idle(&mut host) {
        guest.receive_message(message, HERE);
    }
    let queued = guest.tick(SlotState::Idle, HERE, false, 200).unwrap();
    assert_eq!(queued.name, NAME_IN_NO_WINDOW_LIST);
}
