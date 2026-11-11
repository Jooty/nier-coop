use replicant_game::callback_gate::{
    CALLBACK_GATE_GUEST_GIVE_UP_MS, CallbackGate, GateRole, LocalFire, PartnerFire,
};
use replicant_game::fight_sync::{
    GuestArrival, GuestFightStream, GuestNotice, GuestOpen, GuestPop, GuestReceived,
    HostFightStream, HostStreamOut, PopSource, Replayer, SlotState, StartEntry, StartStreamPop,
    StreamEntry, TapeValue, WindowArming,
};
use replicant_proto::{FightWindowChange, Message, StoryCallArg, decode, encode};

const PACK: &str = "A_CLIFF_VILLAGE_01";
const PHASE: u16 = 2;
const HERE: Option<u16> = Some(PHASE);
const FIGHT_START: &str = "EID_1000_a0035_01_Start";
const FIRST_FIGHT_POP: &str = "Reset_NotAttack_Timer_03";
const SECOND_FIGHT_POP: &str = "EID_1000_a0035_01_End";
const STREAM_SLOT: u64 = 0x1_4130_2070;
const OTHER_SLOT: u64 = 0x1_4130_2818;
const TAPED_NATIVE_THE_START_READS: &str = "_GetUniqueActorMaxLife";
const STONE_FIELD_PACK: &str = "A_STONE_FIELD_02";
const STONE_FIELD_PHASE: u16 = 23;
const STONE_FIELD_ARM_POP: &str = "EID_0200_a0030g_Talk01";

fn over_the_wire(message: Message) -> Message {
    assert_eq!(message.validate(), Ok(()));
    decode(&encode(&message).unwrap()).unwrap()
}

fn host_with_the_start_popped() -> HostFightStream {
    let mut host = HostFightStream::new();
    host.enter_phase(PHASE, PACK);
    let armed = host.popped(FIGHT_START, vec![StoryCallArg::Int(7)], false);
    assert!(
        matches!(
            armed.as_slice(),
            [HostStreamOut::Armed {
                by: FIGHT_START,
                arming: WindowArming::StartId,
                ..
            }]
        ),
        "{armed:?}"
    );
    host
}

fn host_fight() -> Vec<Message> {
    let mut host = host_with_the_start_popped();
    let mut outs = host.callback_registered();
    outs.extend(host.script_slot(SlotState::Idle));
    for name in [FIRST_FIGHT_POP, SECOND_FIGHT_POP] {
        outs.extend(host.popped(name, Vec::new(), false));
        outs.extend(host.script_slot(SlotState::Idle));
    }
    outs.iter()
        .filter_map(HostStreamOut::message)
        .map(over_the_wire)
        .collect()
}

fn guest_given(guest: &mut GuestFightStream, messages: Vec<Message>) -> Vec<GuestReceived> {
    messages
        .into_iter()
        .map(|message| guest.receive_message(message, HERE))
        .collect()
}

fn guest_in_the_hosts_fight() -> GuestFightStream {
    let mut guest = GuestFightStream::new();
    let received = guest_given(&mut guest, host_fight());
    assert!(matches!(
        received.first(),
        Some(GuestReceived::Opened {
            open: GuestOpen::Joined(_),
            ..
        })
    ));
    guest
}

fn released(guest: &mut GuestFightStream, now_ms: u64) -> Option<String> {
    let entry = guest.tick(SlotState::Idle, HERE, false, now_ms)?;
    guest.entry_queued_at(STREAM_SLOT);
    Some(entry.name)
}

fn run_the_start_from_the_stream(guest: &mut GuestFightStream) {
    assert_eq!(released(guest, 0).as_deref(), Some(FIGHT_START));
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, STREAM_SLOT, HERE, false),
        GuestPop::RunStartEntry
    );
    assert_eq!(guest.tick(SlotState::Busy, HERE, false, 16), None);
}

#[test]
fn a_popped_start_is_entry_0_of_the_window_it_opens() {
    let mut host = host_with_the_start_popped();
    let opened = host.callback_registered();
    assert!(
        matches!(
            opened.as_slice(),
            [
                HostStreamOut::Opened {
                    armed_by: Some(FIGHT_START),
                    ..
                },
                HostStreamOut::Entry(StreamEntry { seq: 0, name, args, .. })
            ] if name == FIGHT_START && args == &[StoryCallArg::Int(7)]
        ),
        "{opened:?}"
    );
    let open_message = opened[0].message().map(over_the_wire);
    assert!(matches!(
        open_message,
        Some(Message::FightWindow(mark))
            if mark.change == FightWindowChange::Opened { armed_by: Some(FIGHT_START.to_owned()) }
    ));
    assert_eq!(host.streamed_copies(FIGHT_START), 1);
    let next = host.popped(FIRST_FIGHT_POP, Vec::new(), false);
    assert!(
        matches!(
            next.as_slice(),
            [HostStreamOut::Entry(StreamEntry { seq: 1, .. })]
        ),
        "{next:?}"
    );
}

#[test]
fn the_hosts_start_entry_is_not_taped_and_has_no_end_mark() {
    // The handler began before the window opened, so its reads cannot be numbered from 0.
    let mut host = host_with_the_start_popped();
    host.callback_registered();
    assert_eq!(host.tape_seq(), None);
    assert_eq!(
        host.taped_read(TAPED_NATIVE_THE_START_READS, TapeValue::Int(900)),
        None
    );
    assert_eq!(host.script_slot(SlotState::Idle), None);
}

#[test]
fn a_start_the_hook_only_traced_is_not_streamed() {
    let mut host = HostFightStream::new();
    host.enter_phase(PHASE, PACK);
    host.function_ran(FIGHT_START);
    assert!(host.is_armed());
    let opened = host.callback_registered();
    assert!(
        matches!(
            opened.as_slice(),
            [HostStreamOut::Opened { armed_by: None, .. }]
        ),
        "{opened:?}"
    );
}

#[test]
fn a_player_local_start_pop_is_not_streamed() {
    let mut host = HostFightStream::new();
    host.enter_phase(PHASE, PACK);
    host.popped(FIGHT_START, Vec::new(), true);
    let opened = host.callback_registered();
    assert!(
        matches!(
            opened.as_slice(),
            [HostStreamOut::Opened { armed_by: None, .. }]
        ),
        "{opened:?}"
    );
}

#[test]
fn a_start_pop_of_a_disarmed_window_is_not_streamed_when_a_trace_arms_it_again() {
    let mut host = host_with_the_start_popped();
    assert_eq!(host.restart(), Vec::new());
    assert!(!host.is_armed());
    host.function_ran(FIGHT_START);
    let opened = host.callback_registered();
    assert!(
        matches!(
            opened.as_slice(),
            [HostStreamOut::Opened { armed_by: None, .. }]
        ),
        "{opened:?}"
    );
}

#[test]
fn the_host_names_the_start_ids_of_its_pack() {
    let mut host = HostFightStream::new();
    host.enter_phase(PHASE, PACK);
    assert!(host.is_start_id(FIGHT_START));
    assert!(!host.is_start_id(FIRST_FIGHT_POP));
}

#[test]
fn a_guest_with_a_busy_script_slot_runs_the_start_first_once_the_slot_is_free() {
    let mut guest = guest_in_the_hosts_fight();
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::StartEntryExpected { name, ran_here_already: false }] if name == FIGHT_START
    ));
    // A talk of this player's own holds the slot for minutes.
    for tick in 0..10_000 {
        assert_eq!(guest.tick(SlotState::Busy, HERE, false, tick * 16), None);
    }
    // The partner's earlier calls run before anything of the fight.
    assert_eq!(guest.tick(SlotState::Idle, HERE, true, 160_000), None);
    run_the_start_from_the_stream(&mut guest);
    assert_eq!(guest.tape_seq(), None);
    assert_eq!(
        released(&mut guest, 200_000).as_deref(),
        Some(FIRST_FIGHT_POP)
    );
    assert_eq!(
        guest.pop(FIRST_FIGHT_POP, PopSource::Queued, STREAM_SLOT, HERE, false),
        GuestPop::RunReplayed { seq: 1 }
    );
    assert_eq!(
        released(&mut guest, 200_100).as_deref(),
        Some(SECOND_FIGHT_POP)
    );
}

#[test]
fn the_start_entry_does_not_wait_for_an_end_mark_the_host_never_sends() {
    let mut host = host_with_the_start_popped();
    let messages: Vec<Message> = host
        .callback_registered()
        .iter()
        .filter_map(HostStreamOut::message)
        .map(over_the_wire)
        .collect();
    let mut guest = GuestFightStream::new();
    guest_given(&mut guest, messages);
    assert_eq!(released(&mut guest, 0).as_deref(), Some(FIGHT_START));
}

#[test]
fn the_start_entry_waits_while_a_scene_of_the_guests_own_plays() {
    let mut guest = guest_in_the_hosts_fight();
    guest.set_own_scene_playing(true);
    for tick in 0..1_000 {
        assert_eq!(guest.tick(SlotState::Idle, HERE, false, tick * 16), None);
    }
    // The scene's own end pops the start on this game; the stream's copy is the one that runs.
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::Hold
    );
    guest.set_own_scene_playing(false);
    run_the_start_from_the_stream(&mut guest);
}

#[test]
fn only_the_start_entry_waits_for_the_guests_own_scene() {
    let mut guest = guest_in_the_hosts_fight();
    run_the_start_from_the_stream(&mut guest);
    guest.set_own_scene_playing(true);
    assert_eq!(released(&mut guest, 100).as_deref(), Some(FIRST_FIGHT_POP));
}

#[test]
fn a_start_the_guests_game_ran_before_the_window_is_skipped() {
    let mut guest = GuestFightStream::new();
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::Outside
    );
    guest.ran_here(FIGHT_START, HERE);
    let received = guest_given(&mut guest, host_fight());
    assert!(received.contains(&GuestReceived::Entry {
        seq: 0,
        arrival: GuestArrival::ArmingEntrySkipped,
    }));
    let notices = guest.take_notices();
    assert!(
        matches!(
            notices.as_slice(),
            [
                GuestNotice::StartEntryExpected { ran_here_already: true, .. },
                GuestNotice::ArmingEntrySkipped { name, .. }
            ] if name == FIGHT_START
        ),
        "{notices:?}"
    );
    assert_eq!(released(&mut guest, 0).as_deref(), Some(FIRST_FIGHT_POP));
}

#[test]
fn a_start_pop_a_gate_held_before_the_window_still_runs_from_the_stream() {
    // The pop was judged but a gate held it: it never ran, so the stream's copy must.
    let mut guest = GuestFightStream::new();
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::Outside
    );
    guest_given(&mut guest, host_fight());
    run_the_start_from_the_stream(&mut guest);
}

#[test]
fn a_start_that_ran_in_another_phase_does_not_skip_this_fights_start() {
    let mut guest = GuestFightStream::new();
    guest.ran_here(FIGHT_START, Some(PHASE + 1));
    guest_given(&mut guest, host_fight());
    run_the_start_from_the_stream(&mut guest);
}

#[test]
fn another_copy_that_runs_while_the_start_entry_waits_replaces_it() {
    let mut guest = guest_in_the_hosts_fight();
    guest.take_notices();
    guest.partner_call_queued(FIGHT_START, 0);
    assert_eq!(guest.tick(SlotState::Idle, HERE, false, 16), None);
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::RunPartnerCall
    );
    guest.ran_here(FIGHT_START, HERE);
    assert!(matches!(
        guest.take_notices().as_slice(),
        [GuestNotice::ArmingEntrySkipped { name, .. }] if name == FIGHT_START
    ));
    assert_eq!(guest.tick(SlotState::Busy, HERE, false, 32), None);
    assert_eq!(released(&mut guest, 48).as_deref(), Some(FIRST_FIGHT_POP));
}

#[test]
fn a_partner_copy_a_gate_held_leaves_the_start_to_the_stream() {
    let mut guest = guest_in_the_hosts_fight();
    guest.partner_call_queued(FIGHT_START, 0);
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::RunPartnerCall
    );
    // No `ran_here`: the gate held that pop.
    run_the_start_from_the_stream(&mut guest);
}

#[test]
fn another_copy_that_runs_before_entry_0_arrives_skips_it_on_arrival() {
    let mut messages = host_fight();
    let rest = messages.split_off(1);
    let mut guest = GuestFightStream::new();
    guest_given(&mut guest, messages);
    guest.ran_here(FIGHT_START, HERE);
    let received = guest_given(&mut guest, rest);
    assert!(received.contains(&GuestReceived::Entry {
        seq: 0,
        arrival: GuestArrival::ArmingEntrySkipped,
    }));
    assert_eq!(released(&mut guest, 0).as_deref(), Some(FIRST_FIGHT_POP));
}

#[test]
fn the_streams_copy_is_not_run_when_another_copy_ran_while_it_sat_in_the_queue() {
    let mut guest = guest_in_the_hosts_fight();
    assert_eq!(released(&mut guest, 0).as_deref(), Some(FIGHT_START));
    guest.ran_here(FIGHT_START, HERE);
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, STREAM_SLOT, HERE, false),
        GuestPop::StartAlreadyRan
    );
    assert_eq!(released(&mut guest, 16).as_deref(), Some(FIRST_FIGHT_POP));
}

#[test]
fn no_other_copy_of_the_start_runs_after_the_stream_ran_it() {
    let mut guest = guest_in_the_hosts_fight();
    run_the_start_from_the_stream(&mut guest);
    guest.partner_call_queued(FIGHT_START, 32);
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::StartAlreadyRan
    );
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Fired, OTHER_SLOT, HERE, false),
        GuestPop::Hold
    );
    // The stream's own run is traced too, and changes nothing.
    guest.ran_here(FIGHT_START, HERE);
    assert_eq!(released(&mut guest, 64).as_deref(), Some(FIRST_FIGHT_POP));
}

#[test]
fn a_partner_call_of_another_name_still_runs_inside_the_window() {
    let mut guest = guest_in_the_hosts_fight();
    run_the_start_from_the_stream(&mut guest);
    guest.partner_call_queued(SECOND_FIGHT_POP, 32);
    assert_eq!(
        guest.pop(SECOND_FIGHT_POP, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::RunPartnerCall
    );
}

#[test]
fn a_window_an_arm_id_opened_keeps_its_own_rule() {
    let mut host = HostFightStream::new();
    host.enter_phase(STONE_FIELD_PHASE, STONE_FIELD_PACK);
    let messages: Vec<Message> = host
        .popped(STONE_FIELD_ARM_POP, Vec::new(), false)
        .iter()
        .filter_map(HostStreamOut::message)
        .map(over_the_wire)
        .collect();
    let here = Some(STONE_FIELD_PHASE);
    let mut guest = GuestFightStream::new();
    for message in messages {
        guest.receive_message(message, here);
    }
    assert_eq!(guest.take_notices(), Vec::new());
    guest.set_own_scene_playing(true);
    guest.ran_here(STONE_FIELD_ARM_POP, here);
    let entry = guest.tick(SlotState::Idle, here, false, 0);
    assert!(
        matches!(entry, Some(StreamEntry { seq: 0, .. })),
        "{entry:?}"
    );
    guest.entry_queued_at(STREAM_SLOT);
    assert_eq!(
        guest.pop(
            STONE_FIELD_ARM_POP,
            PopSource::Queued,
            STREAM_SLOT,
            here,
            false
        ),
        GuestPop::RunReplayed { seq: 0 }
    );
}

#[test]
fn a_restart_of_either_game_forgets_the_start_entry() {
    let mut guest = guest_in_the_hosts_fight();
    guest.take_notices();
    guest.own_restart(0);
    guest.partner_call_queued(FIGHT_START, 0);
    assert_eq!(
        guest.pop(FIGHT_START, PopSource::Queued, OTHER_SLOT, HERE, false),
        GuestPop::RunPartnerCall
    );
    guest.ran_here(FIGHT_START, HERE);
    assert_eq!(guest.take_notices(), Vec::new());
}

#[test]
fn the_start_entry_runs_once_from_whichever_copy_is_first() {
    let mut entry = StartEntry::new();
    assert!(!entry.is_expected(FIGHT_START));
    assert_eq!(entry.stream_copy_popped(FIGHT_START), None);

    entry.expect(FIGHT_START, false);
    assert!(entry.is_waiting_for(FIGHT_START));
    assert!(!entry.is_waiting_for(FIRST_FIGHT_POP));
    assert!(!entry.another_copy_ran(FIRST_FIGHT_POP));
    assert_eq!(
        entry.stream_copy_popped(FIGHT_START),
        Some(StartStreamPop::Run)
    );
    assert!(entry.holds_other_copies_of(FIGHT_START));
    assert!(!entry.holds_other_copies_of(FIRST_FIGHT_POP));
    assert!(!entry.another_copy_ran(FIGHT_START));
    assert_eq!(entry.stream_copy_popped(FIGHT_START), None);

    entry.expect(FIGHT_START, false);
    assert!(entry.another_copy_ran(FIGHT_START));
    assert!(entry.skips_stream_copy_of(FIGHT_START));
    assert!(!entry.holds_other_copies_of(FIGHT_START));
    assert_eq!(
        entry.stream_copy_popped(FIGHT_START),
        Some(StartStreamPop::AlreadyRan)
    );
    assert!(!entry.is_expected(FIGHT_START));

    entry.expect(FIGHT_START, true);
    assert!(entry.skips_stream_copy_of(FIGHT_START));
    entry.stream_copy_skipped();
    assert!(!entry.is_expected(FIGHT_START));

    entry.expect(FIGHT_START, false);
    entry.forget();
    assert!(!entry.is_expected(FIGHT_START));
}

#[test]
fn the_start_entry_is_held_back_only_by_the_guests_own_scene() {
    let mut entry = StartEntry::new();
    entry.expect(FIGHT_START, false);
    assert!(entry.may_release(FIGHT_START, false));
    assert!(!entry.may_release(FIGHT_START, true));
    assert!(entry.may_release(FIRST_FIGHT_POP, true));
}

#[test]
fn a_pending_entry_is_skipped_only_when_it_is_next_and_nothing_is_in_flight() {
    let entry = |seq| StreamEntry {
        phase: PHASE,
        epoch: 0,
        seq,
        name: format!("entry {seq}"),
        args: Vec::new(),
    };
    let mut replayer = Replayer::new(Vec::<String>::new());
    assert!(!replayer.skip_pending_entry(0));
    replayer.receive(entry(0));
    replayer.receive(entry(1));
    assert!(!replayer.skip_pending_entry(1));
    assert!(replayer.skip_pending_entry(0));
    assert_eq!(replayer.next_seq(), 1);
    assert_eq!(replayer.tick(SlotState::Idle).map(|next| next.seq), Some(1));
    replayer.receive(entry(2));
    assert!(!replayer.skip_pending_entry(2));
}

#[test]
fn the_gate_runs_no_copy_of_a_start_the_fight_stream_ran() {
    let mut gate = CallbackGate::new();
    gate.note_partner_here(true);
    assert_eq!(
        gate.local_fire(GateRole::Guest, FIGHT_START, 0),
        LocalFire::Hold
    );
    gate.ran_from_the_fight_stream(FIGHT_START);
    assert!(!gate.holds_any());
    assert_eq!(
        gate.step(
            GateRole::Guest,
            false,
            false,
            CALLBACK_GATE_GUEST_GIVE_UP_MS + 1
        ),
        Vec::new()
    );

    let mut gate = CallbackGate::new();
    gate.note_partner_here(false);
    assert_eq!(
        gate.partner_fire(GateRole::Guest, FIGHT_START, 0),
        PartnerFire::Noted
    );
    gate.allow_next(FIGHT_START, 0);
    gate.ran_from_the_fight_stream(FIGHT_START);
    assert!(!gate.partner_led(FIGHT_START, 1));
    assert_eq!(
        gate.local_fire(GateRole::Guest, FIGHT_START, 1),
        LocalFire::RunAlone
    );
}

#[test]
fn the_gate_keeps_copies_of_other_names_when_the_stream_ran_a_start() {
    let mut gate = CallbackGate::new();
    gate.note_partner_here(true);
    assert_eq!(
        gate.local_fire(GateRole::Guest, FIRST_FIGHT_POP, 0),
        LocalFire::Hold
    );
    gate.ran_from_the_fight_stream(FIGHT_START);
    assert!(gate.holds_any());
}
