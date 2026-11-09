use rc_game::fight_sync::{
    ArmedBy, GuestArrival, GuestFightStream, GuestNotice, GuestPop, GuestReceived, HostFightStream,
    HostStreamOut, OWN_POP_HELD_FOR_HOST_COPY_MS, ObjectArmedCalls, PopSource,
    QUEUED_CALL_NAME_CAPACITY, ResultWrite, SlotState, StreamEntry, TAPED_NATIVE_RVAS,
    TAPED_NATIVES, TAPED_NATIVES_HOOKED_BY_SCRIPT_ROLLS, TapeValue, is_our_queued_call,
    is_pop_by_this_player, is_taped_native, queued_call_name,
};
use rc_game::script_rolls::{GET_RAND_F_RVA, GET_RAND_I_RVA};
use rc_game::script_runner::QUEUE_SLOT_COUNT;
use rc_game::story_relay::{ARG_TAG_INT, ARG_TAG_NUMBER, ARG_TAG_TEXT};
use rc_proto::{FightTape, FightTapeValue, Message, decode, encode};

const STONE_FIELD_PACK: &str = "A_STONE_FIELD_02";
const STONE_FIELD_PHASE: u16 = 23;
const FIGHT_ARM_POP: &str = "EID_0200_a0030g_Talk01";
const BOSS_PHASE_ONE: &str = "CB_BossCA_01";
const BOSS_PHASE_TWO: &str = "CB_BossCA_02";
const EVENT_ACTION: &str = "Action_to_WhiteBook";
const HERE: Option<u16> = Some(STONE_FIELD_PHASE);

fn over_the_wire(message: Message) -> Message {
    assert_eq!(message.validate(), Ok(()));
    assert!(message.is_relayed());
    decode(&encode(&message).unwrap()).unwrap()
}

fn open_fight() -> (HostFightStream, GuestFightStream) {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    let opened = host.popped(FIGHT_ARM_POP, Vec::new(), false);
    // The guest's own copy of the arming pop ran first, so it skips the host's entry 0.
    let mut guest = GuestFightStream::new();
    guest.pop(FIGHT_ARM_POP, PopSource::Fired, 0, HERE, false);
    let mut received = opened
        .iter()
        .filter_map(HostStreamOut::message)
        .map(|message| guest.receive_message(over_the_wire(message), HERE));
    assert!(matches!(
        received.next(),
        Some(GuestReceived::Opened { .. })
    ));
    assert!(matches!(
        received.next(),
        Some(GuestReceived::Entry {
            arrival: GuestArrival::ArmingEntrySkipped,
            ..
        })
    ));
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::ArmingEntrySkipped { .. }]
    ));
    (host, guest)
}

fn host_pops(host: &mut HostFightStream, name: &str) -> StreamEntry {
    host.popped(name, Vec::new(), false)
        .into_iter()
        .find_map(|out| match out {
            HostStreamOut::Entry(entry) => Some(entry),
            _ => None,
        })
        .expect("a pop inside the window is streamed")
}

fn host_reads(host: &mut HostFightStream, reads: &[(&str, TapeValue)]) -> Vec<Message> {
    reads
        .iter()
        .map(|(native, value)| {
            host.taped_read(native, *value)
                .expect("a read inside a streamed handler is taped")
                .message
        })
        .collect()
}

fn guest_receives(guest: &mut GuestFightStream, messages: Vec<Message>) {
    for message in messages {
        guest.receive_message(over_the_wire(message), HERE);
    }
}

fn guest_starts(guest: &mut GuestFightStream, entry: &StreamEntry) {
    guest_receives(guest, vec![Message::FightStream(entry.to_wire())]);
    // These tests are about the values; the host's end of the entry is taken as already here.
    assert!(guest.host_entry_ended(entry.phase, entry.epoch, entry.seq));
    let queued = guest
        .tick(SlotState::Idle, HERE, false, 100)
        .expect("the entry is queued");
    assert_eq!(queued.seq, entry.seq);
    assert_eq!(
        guest.pop(&queued.name, PopSource::Queued, 0, HERE, false),
        GuestPop::RunReplayed { seq: entry.seq }
    );
}

#[test]
fn the_tape_list_is_hooked_exactly_with_one_rva_per_native() {
    let hooked: Vec<&str> = TAPED_NATIVE_RVAS
        .iter()
        .map(|(native, _)| *native)
        .collect();
    assert_eq!(hooked, TAPED_NATIVES.to_vec());
    assert!(TAPED_NATIVES.iter().all(|native| is_taped_native(native)));
    let mut rvas: Vec<u32> = TAPED_NATIVE_RVAS.iter().map(|(_, rva)| *rva).collect();
    rvas.sort_unstable();
    rvas.dedup();
    assert_eq!(rvas.len(), TAPED_NATIVES.len());
    let rva_of = |name: &str| {
        TAPED_NATIVE_RVAS
            .iter()
            .find(|(native, _)| *native == name)
            .map(|(_, rva)| *rva)
    };
    assert_eq!(rva_of("_GetRandI"), Some(GET_RAND_I_RVA));
    assert_eq!(rva_of("_GetRandF"), Some(GET_RAND_F_RVA));
    assert!(
        TAPED_NATIVES_HOOKED_BY_SCRIPT_ROLLS
            .iter()
            .all(|native| TAPED_NATIVES.contains(native))
    );
}

#[test]
fn a_native_result_slot_reads_as_int_number_or_missing_and_writes_back_its_own_kind() {
    let int_payload = [7, 0, 0, 0, 0xAA, 0xBB, 0xCC, 0xDD];
    assert_eq!(
        TapeValue::from_result_slot(1, ARG_TAG_INT, int_payload),
        TapeValue::Int(7)
    );
    let number_payload = 0.375_f64.to_le_bytes();
    assert_eq!(
        TapeValue::from_result_slot(1, ARG_TAG_NUMBER, number_payload),
        TapeValue::Float(0.375)
    );
    assert_eq!(
        TapeValue::from_result_slot(0, ARG_TAG_INT, int_payload),
        TapeValue::Missing
    );
    assert_eq!(
        TapeValue::from_result_slot(1, ARG_TAG_TEXT, int_payload),
        TapeValue::Missing
    );
    assert_eq!(
        TapeValue::Int(-2).result_payload(),
        Some((-2_i32).to_le_bytes().to_vec())
    );
    assert_eq!(
        TapeValue::Float(0.375).result_payload(),
        Some(number_payload.to_vec())
    );
    assert_eq!(TapeValue::Int(i64::MAX).result_payload(), None);
    assert_eq!(TapeValue::Missing.result_payload(), None);
    for value in [TapeValue::Int(3), TapeValue::Float(1.5), TapeValue::Missing] {
        assert_eq!(TapeValue::from_wire(value.to_wire()), value);
    }
}

#[test]
fn the_host_tapes_reads_only_inside_a_streamed_handler_numbered_per_native() {
    let (mut host, _) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let messages = host_reads(
        &mut host,
        &[
            ("_GetActorLife", TapeValue::Int(120)),
            ("_GetRandI", TapeValue::Int(2)),
            ("_GetActorLife", TapeValue::Int(80)),
        ],
    );
    let tapes: Vec<FightTape> = messages
        .into_iter()
        .map(|message| match message {
            Message::FightTape(tape) => tape,
            other => panic!("a tape message, not {other:?}"),
        })
        .collect();
    assert!(tapes.iter().all(|tape| tape.seq == entry.seq
        && tape.epoch == entry.epoch
        && tape.phase == STONE_FIELD_PHASE));
    let numbered: Vec<(&str, u32, FightTapeValue)> = tapes
        .iter()
        .map(|tape| (tape.native.as_str(), tape.call_index, tape.value))
        .collect();
    assert_eq!(
        numbered,
        vec![
            ("_GetActorLife", 0, FightTapeValue::Int(120)),
            ("_GetRandI", 0, FightTapeValue::Int(2)),
            ("_GetActorLife", 1, FightTapeValue::Int(80)),
        ]
    );
    host.script_slot(SlotState::Busy);
    assert!(host.taped_read("_GetRandI", TapeValue::Int(1)).is_some());
    host.script_slot(SlotState::Idle);
    assert_eq!(host.taped_read("_GetRandI", TapeValue::Int(1)), None);
    host.popped("LIB_GameOver", Vec::new(), false);
    assert_eq!(host.taped_read("_GetRandI", TapeValue::Int(1)), None);
}

#[test]
fn the_first_host_read_of_each_native_in_an_entry_logs_its_native_and_seq() {
    let (mut host, _) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let first = host
        .taped_read("_GetGlobalTimeCount", TapeValue::Float(12.5))
        .unwrap();
    let line = first.log.expect("the first read logs");
    assert!(line.contains("_GetGlobalTimeCount") && line.contains(&format!("seq {}", entry.seq)));
    let again = host
        .taped_read("_GetGlobalTimeCount", TapeValue::Float(12.6))
        .unwrap();
    assert_eq!(again.log, None);
}

#[test]
fn reads_outside_an_open_window_are_not_taped_on_the_host() {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    host.popped(BOSS_PHASE_ONE, Vec::new(), false);
    assert_eq!(host.taped_read("_GetRandI", TapeValue::Int(4)), None);
}

#[test]
fn the_guest_replay_takes_the_host_values_read_by_read_and_logs_the_first_hit() {
    let (mut host, mut guest) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let tapes = host_reads(
        &mut host,
        &[
            ("_GetActorLife", TapeValue::Int(120)),
            ("_GetActorLife", TapeValue::Int(80)),
            ("_GetActorLifeRate", TapeValue::Float(0.25)),
        ],
    );
    guest_starts(&mut guest, &entry);
    guest_receives(&mut guest, tapes);
    let first = guest
        .read_taped("_GetActorLife", TapeValue::Int(500), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(first.value, TapeValue::Int(120));
    let line = first.log.expect("the first hit logs");
    assert!(line.contains("_GetActorLife") && line.contains(&format!("seq {}", entry.seq)));
    let second = guest
        .read_taped("_GetActorLife", TapeValue::Int(500), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!((second.value, second.log), (TapeValue::Int(80), None));
    let rate = guest
        .read_taped(
            "_GetActorLifeRate",
            TapeValue::Float(1.0),
            HERE,
            SlotState::Busy,
        )
        .unwrap();
    assert_eq!(rate.value, TapeValue::Float(0.25));
    // The handler ended: the next read is the game's own again.
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 200), None);
    assert_eq!(
        guest.read_taped("_GetActorLife", TapeValue::Int(500), HERE, SlotState::Busy),
        None
    );
}

#[test]
fn a_tape_that_arrives_before_its_entry_starts_is_kept_for_it() {
    let (mut host, mut guest) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let tapes = host_reads(&mut host, &[("_GetRandI", TapeValue::Int(5))]);
    guest_receives(&mut guest, tapes);
    guest_starts(&mut guest, &entry);
    let read = guest
        .read_taped("_GetRandI", TapeValue::Int(0), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(read.value, TapeValue::Int(5));
}

#[test]
fn each_replayed_entry_reads_only_its_own_tape() {
    let (mut host, mut guest) = open_fight();
    let one = host_pops(&mut host, BOSS_PHASE_ONE);
    let mut tapes = host_reads(&mut host, &[("_GetRandI", TapeValue::Int(1))]);
    host.script_slot(SlotState::Idle);
    let two = host_pops(&mut host, BOSS_PHASE_TWO);
    tapes.extend(host_reads(&mut host, &[("_GetRandI", TapeValue::Int(2))]));
    guest_receives(&mut guest, tapes);
    guest_starts(&mut guest, &one);
    let first = guest
        .read_taped("_GetRandI", TapeValue::Int(9), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(first.value, TapeValue::Int(1));
    guest_starts(&mut guest, &two);
    let second = guest
        .read_taped("_GetRandI", TapeValue::Int(9), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(second.value, TapeValue::Int(2));
}

#[test]
fn a_host_value_of_another_kind_is_used_and_a_missing_one_keeps_the_local_value_with_one_line() {
    // The host's actor is gone (no result) while the guest's still answers: the guest takes the
    // host's missing value, and the hook rewrites the result count to match.
    let (mut host, mut guest) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let tapes = host_reads(&mut host, &[("_GetActorLife", TapeValue::Missing)]);
    guest_starts(&mut guest, &entry);
    guest_receives(&mut guest, tapes);
    let mismatch = guest
        .read_taped("_GetActorLife", TapeValue::Int(300), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(mismatch.value, TapeValue::Missing);
    assert_eq!(
        mismatch.value.write_over(TapeValue::Int(300)),
        ResultWrite::Write {
            result_count: Some(0),
            slot: None
        }
    );
    let line = mismatch.log.expect("a kind change logs");
    assert!(line.contains("_GetActorLife") && line.contains(&format!("seq {}", entry.seq)));
    let untaped = guest
        .read_taped("_GetActorLife", TapeValue::Int(300), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!((untaped.value, untaped.log), (TapeValue::Int(300), None));
    let missing = guest
        .read_taped("_GetActorCount", TapeValue::Int(2), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(missing.value, TapeValue::Int(2));
    assert!(missing.log.unwrap().contains("_GetActorCount"));
}

#[test]
fn both_games_missing_the_actor_read_as_missing_and_keep_the_call_numbers_aligned() {
    let (mut host, mut guest) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let tapes = host_reads(
        &mut host,
        &[
            ("_GetActorLife", TapeValue::Missing),
            ("_GetActorLife", TapeValue::Int(40)),
        ],
    );
    guest_starts(&mut guest, &entry);
    guest_receives(&mut guest, tapes);
    let first = guest
        .read_taped("_GetActorLife", TapeValue::Missing, HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(first.value, TapeValue::Missing);
    let second = guest
        .read_taped("_GetActorLife", TapeValue::Int(400), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(second.value, TapeValue::Int(40));
}

#[test]
fn a_tape_of_another_epoch_or_phase_is_not_kept() {
    let (mut host, mut guest) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let Message::FightTape(tape) =
        host_reads(&mut host, &[("_GetRandI", TapeValue::Int(5))]).remove(0)
    else {
        panic!("a tape message");
    };
    let old_epoch = FightTape {
        epoch: tape.epoch - 1,
        ..tape.clone()
    };
    let other_phase = FightTape {
        phase: STONE_FIELD_PHASE + 1,
        ..tape.clone()
    };
    for stale in [old_epoch, other_phase] {
        assert_eq!(
            guest.receive_message(over_the_wire(Message::FightTape(stale)), HERE),
            GuestReceived::Tape {
                seq: entry.seq,
                kept: false
            }
        );
    }
    let mut outside = GuestFightStream::new();
    assert_eq!(
        outside.receive_message(over_the_wire(Message::FightTape(tape)), HERE),
        GuestReceived::Tape {
            seq: entry.seq,
            kept: false
        }
    );
    guest_starts(&mut guest, &entry);
    let read = guest
        .read_taped("_GetRandI", TapeValue::Int(0), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(read.value, TapeValue::Int(0));
}

#[test]
fn a_host_restart_drops_the_tape_of_the_old_epoch() {
    let (mut host, mut guest) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let tapes = host_reads(&mut host, &[("_GetRandI", TapeValue::Int(5))]);
    guest_receives(&mut guest, tapes);
    let restarted = host.restart();
    assert_eq!(host.taped_read("_GetRandI", TapeValue::Int(5)), None);
    let mark = restarted
        .iter()
        .find_map(HostStreamOut::message)
        .expect("the restart is sent");
    guest_receives(&mut guest, vec![mark]);
    guest.own_restart(100);
    let replayed = host_pops(&mut host, BOSS_PHASE_ONE);
    assert_eq!((entry.seq, replayed.seq), (1, 0));
    guest_starts(&mut guest, &replayed);
    let read = guest
        .read_taped("_GetRandI", TapeValue::Int(0), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(read.value, TapeValue::Int(0));
}

#[test]
fn reads_on_the_guest_outside_a_replayed_handler_or_its_fight_stay_local() {
    let (_, mut guest) = open_fight();
    assert_eq!(
        guest.read_taped("_GetRandI", TapeValue::Int(3), HERE, SlotState::Busy),
        None
    );
    let mut outside = GuestFightStream::new();
    assert_eq!(
        outside.read_taped("_GetRandI", TapeValue::Int(3), HERE, SlotState::Busy),
        None
    );
}

#[test]
fn a_queued_call_a_game_object_armed_is_known_at_its_pop_once() {
    let mut armed = ObjectArmedCalls::new();
    armed.armed(0x1000, EVENT_ACTION, ArmedBy::GameObject);
    armed.armed(0x2000, BOSS_PHASE_ONE, ArmedBy::ScriptOrRelay);
    assert_eq!(armed.len(), 1);
    assert!(armed.popped(0x1000, EVENT_ACTION));
    assert!(!armed.popped(0x1000, EVENT_ACTION));
    assert!(!armed.popped(0x2000, BOSS_PHASE_ONE));
    armed.armed(0x1000, EVENT_ACTION, ArmedBy::GameObject);
    assert!(
        !armed.popped(0x1000, BOSS_PHASE_ONE),
        "another call in the slot"
    );
    armed.armed(0x3000, EVENT_ACTION, ArmedBy::GameObject);
    armed.armed(0x3000, BOSS_PHASE_TWO, ArmedBy::ScriptOrRelay);
    assert!(
        !armed.popped(0x3000, BOSS_PHASE_TWO),
        "the slot was reused by a script"
    );
    for slot in 0..=QUEUE_SLOT_COUNT {
        armed.armed(slot, EVENT_ACTION, ArmedBy::GameObject);
    }
    assert_eq!(armed.len(), QUEUE_SLOT_COUNT as usize);
    assert!(!armed.popped(0, EVENT_ACTION), "the oldest is dropped");
    armed.clear();
    assert!(armed.is_empty());
}

#[test]
fn an_event_action_or_gimmick_pop_on_the_guest_goes_to_the_host_once_like_a_trap() {
    assert!(is_pop_by_this_player(false, true));
    assert!(is_pop_by_this_player(true, false));
    assert!(!is_pop_by_this_player(false, false));
    let (mut host, mut guest) = open_fight();
    let mut armed = ObjectArmedCalls::new();
    armed.armed(0x1048, EVENT_ACTION, ArmedBy::GameObject);
    assert!(is_pop_by_this_player(
        false,
        armed.popped(0x1048, EVENT_ACTION)
    ));
    assert_eq!(
        guest.pop_by_player(EVENT_ACTION, Vec::new(), HERE, false, 100),
        GuestPop::HeldForHostCopy
    );
    // The player takes the same action again before any host copy came.
    assert_eq!(
        guest.pop_by_player(EVENT_ACTION, Vec::new(), HERE, false, 200),
        GuestPop::Hold
    );
    guest.tick(
        SlotState::Busy,
        HERE,
        false,
        100 + OWN_POP_HELD_FOR_HOST_COPY_MS + 1,
    );
    let requests = guest.take_requests();
    assert_eq!(requests.len(), 1);
    let Message::FightRequest(request) = over_the_wire(Message::FightRequest(requests[0].clone()))
    else {
        panic!("a request");
    };
    assert!(matches!(
        host.receive_request(request),
        rc_game::fight_sync::HostRequestOut::Run(_)
    ));
    let call = host
        .tick_requests(SlotState::Idle, false)
        .expect("the request is queued on the host");
    assert_eq!(call.name, EVENT_ACTION);
    // A queued call the guest's own replayed script armed is not the player's: it stays held.
    assert!(!is_pop_by_this_player(
        false,
        armed.popped(0x2048, BOSS_PHASE_ONE)
    ));
    assert_eq!(
        guest.pop(BOSS_PHASE_ONE, PopSource::Queued, 0, HERE, false),
        GuestPop::Hold
    );
}

#[test]
fn a_queue_slot_name_reads_up_to_its_nul() {
    let mut bytes = [0_u8; QUEUED_CALL_NAME_CAPACITY];
    bytes[..EVENT_ACTION.len()].copy_from_slice(EVENT_ACTION.as_bytes());
    assert_eq!(queued_call_name(&bytes).as_deref(), Some(EVENT_ACTION));
    assert_eq!(queued_call_name(&[0_u8; 4]), None);
    assert_eq!(queued_call_name(&[0xFF, 0xFE, 0]), None);
}

#[test]
fn a_read_with_the_script_slot_idle_belongs_to_no_entry_and_ends_the_tape() {
    // F7a 10: the tape used to end only at the next pop or idle tick, so a read between the
    // handler's end and that tick was counted against the finished entry.
    let (mut host, mut guest) = open_fight();
    let entry = host_pops(&mut host, BOSS_PHASE_ONE);
    let tapes = host_reads(
        &mut host,
        &[
            ("_GetActorLife", TapeValue::Int(120)),
            ("_GetActorLife", TapeValue::Int(80)),
        ],
    );
    guest_starts(&mut guest, &entry);
    guest_receives(&mut guest, tapes);
    let inside = guest
        .read_taped("_GetActorLife", TapeValue::Int(500), HERE, SlotState::Busy)
        .unwrap();
    assert_eq!(inside.value, TapeValue::Int(120));
    assert_eq!(
        guest.read_taped("_GetActorLife", TapeValue::Int(500), HERE, SlotState::Idle),
        None
    );
    assert_eq!(guest.tape_seq(), None);
    assert_eq!(
        guest.read_taped("_GetActorLife", TapeValue::Int(500), HERE, SlotState::Busy),
        None,
        "the host's second value is not used by a read after the handler ended"
    );
}

#[test]
fn a_host_value_is_written_over_the_result_count_and_slot_whatever_kind_either_is() {
    assert_eq!(
        TapeValue::Int(3).write_over(TapeValue::Int(3)),
        ResultWrite::Nothing
    );
    assert_eq!(
        TapeValue::Int(-2).write_over(TapeValue::Missing),
        ResultWrite::Write {
            result_count: Some(1),
            slot: Some((ARG_TAG_INT, [0xFE, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]))
        }
    );
    assert_eq!(
        TapeValue::Float(0.5).write_over(TapeValue::Int(1)),
        ResultWrite::Write {
            result_count: None,
            slot: Some((ARG_TAG_NUMBER, 0.5_f64.to_le_bytes()))
        }
    );
    assert_eq!(
        TapeValue::Missing.write_over(TapeValue::Float(1.0)),
        ResultWrite::Write {
            result_count: Some(0),
            slot: None
        }
    );
    assert_eq!(
        TapeValue::Int(i64::MAX).write_over(TapeValue::Int(1)),
        ResultWrite::Unwritable
    );
    // Written back, the slot reads as the host's value.
    let ResultWrite::Write {
        slot: Some((tag, payload)),
        ..
    } = TapeValue::Int(-2).write_over(TapeValue::Missing)
    else {
        panic!("a slot write");
    };
    assert_eq!(
        TapeValue::from_result_slot(1, tag, payload),
        TapeValue::Int(-2)
    );
}

#[test]
fn a_queue_slot_is_ours_only_while_ready_and_under_our_name() {
    assert!(is_our_queued_call(
        Some(2),
        Some(BOSS_PHASE_ONE),
        BOSS_PHASE_ONE
    ));
    assert!(!is_our_queued_call(
        Some(0),
        Some(BOSS_PHASE_ONE),
        BOSS_PHASE_ONE
    ));
    assert!(!is_our_queued_call(
        Some(1),
        Some(BOSS_PHASE_ONE),
        BOSS_PHASE_ONE
    ));
    assert!(
        !is_our_queued_call(Some(2), Some(BOSS_PHASE_TWO), BOSS_PHASE_ONE),
        "the slot was popped and taken by another call"
    );
    assert!(!is_our_queued_call(None, None, BOSS_PHASE_ONE));
}
