use rc_game::callback_gate::{EndFollow, EndQueue, ParkedCall, ParkedKind, QueueEvent};
use rc_game::fight_sync::SlotState;
use rc_game::script_functions::{FillInReach, FillInRelay};
use rc_game::story_relay::{
    RELAY_CALL_POP_EXPECTED_MS, RelayCallFlight, RelayCallGate, RelayCallNotice,
};
use rc_proto::StoryCall;

const STONE_FIELD_PHASE: u16 = 23;
const BOOK_SCENE_END: &str = "EID_0200_a0030g_End";
const BOOK_TALK_02: &str = "EID_0200_a0030g_Talk02";
const BOOK_TALK_03: &str = "EID_0200_a0030g_Talk03";
const WAVE_END: &str = "EID_0200_a0037g_02_End";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Path {
    EndQueue,
    MessageHandler,
    FillIn,
}

fn call(name: &str) -> StoryCall {
    StoryCall {
        phase: STONE_FIELD_PHASE,
        pack: 0,
        function: name.to_owned(),
        args: Vec::new(),
    }
}

fn never_modal(_: &StoryCall) -> bool {
    false
}

struct Engine {
    queued: Vec<(u64, String)>,
    next_slot: u64,
    ran: Vec<String>,
    most_queued_at_once: usize,
}

impl Engine {
    fn new() -> Self {
        Self {
            queued: Vec::new(),
            next_slot: 0x48,
            ran: Vec::new(),
            most_queued_at_once: 0,
        }
    }

    fn queue(&mut self, name: &str) -> u64 {
        let slot = self.next_slot;
        self.next_slot += 0x7A8;
        self.queued.push((slot, name.to_owned()));
        self.most_queued_at_once = self.most_queued_at_once.max(self.queued.len());
        slot
    }

    fn pop_and_run(&mut self) -> Option<u64> {
        if self.queued.is_empty() {
            return None;
        }
        let (slot, name) = self.queued.remove(0);
        self.ran.push(name);
        Some(slot)
    }
}

fn run_until_drained<T>(gate: &mut RelayCallGate<T>, engine: &mut Engine, from_ms: u64) {
    for now in (from_ms..from_ms + 1_000).step_by(16) {
        if let Some(due) = gate.tick(SlotState::Idle, false, now, never_modal) {
            let slot = engine.queue(&due.call.function);
            gate.queued(&due.call.function, slot, now);
        }
        if let Some(slot) = engine.pop_and_run() {
            assert!(gate.popped(slot));
        }
    }
}

#[test]
fn two_paths_with_calls_ready_at_once_never_have_two_queued_and_keep_arrival_order() {
    // The end queue used to release every parked call in one tick (VP-B56) while a message
    // handler from the host came in beside them.
    let mut ends = EndQueue::new();
    for name in [BOOK_TALK_02, BOOK_TALK_03] {
        ends.park_call_always(
            ParkedCall {
                kind: ParkedKind::Function,
                call: call(name),
            },
            0,
        );
    }
    let released: Vec<StoryCall> = ends
        .advance(10, |_| EndFollow::Mirror)
        .into_iter()
        .filter_map(|event| match event {
            QueueEvent::RunCall { parked, .. } => Some(parked.call),
            _ => None,
        })
        .collect();
    assert_eq!(released.len(), 2, "both parked calls are released at once");
    let mut gate = RelayCallGate::new();
    gate.submit(released[0].clone(), Path::EndQueue, 10);
    gate.submit(call(BOOK_SCENE_END), Path::MessageHandler, 10);
    gate.submit(released[1].clone(), Path::EndQueue, 10);
    let mut engine = Engine::new();
    run_until_drained(&mut gate, &mut engine, 10);
    assert_eq!(engine.most_queued_at_once, 1);
    assert_eq!(engine.ran, [BOOK_TALK_02, BOOK_SCENE_END, BOOK_TALK_03]);
    assert!(gate.is_clear());
}

#[test]
fn the_next_call_waits_for_the_pop_and_for_the_slot_to_go_idle_again() {
    let mut gate = RelayCallGate::new();
    gate.submit(call(WAVE_END), Path::EndQueue, 0);
    gate.submit(call(BOOK_SCENE_END), Path::MessageHandler, 0);
    let first = gate.tick(SlotState::Idle, false, 0, never_modal).unwrap();
    assert_eq!(first.call.function, WAVE_END);
    gate.queued(WAVE_END, 0x48, 0);
    assert_eq!(
        gate.tick(SlotState::Idle, false, 16, never_modal),
        None,
        "queued, not popped"
    );
    assert!(!gate.popped(0x48 + 0x7A8), "another slot's pop is not ours");
    assert!(gate.popped(0x48));
    assert_eq!(gate.tick(SlotState::Busy, false, 32, never_modal), None);
    assert_eq!(
        gate.tick(SlotState::SteppingHeld, false, 48, never_modal),
        None,
        "held, not ended"
    );
    let second = gate.tick(SlotState::Idle, false, 64, never_modal).unwrap();
    assert_eq!(second.call.function, BOOK_SCENE_END);
    assert_eq!(second.then, Path::MessageHandler);
}

#[test]
fn a_dropped_fill_in_copy_no_longer_stalls_the_next_one() {
    // Before: a forced drain's own copy that was dropped kept the fill-in relay "queued" until
    // its 5 s timeout. Now a copy that never reaches the gate leaves it clear.
    let mut relay = FillInRelay::new();
    let gate: RelayCallGate<Path> = RelayCallGate::new();
    relay.arrive(
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        FillInReach::CannotReachHere,
        0,
    );
    relay.arrive(
        STONE_FIELD_PHASE,
        BOOK_TALK_02,
        FillInReach::CannotReachHere,
        0,
    );
    let first = relay.tick(gate.is_clear(), 0).unwrap();
    assert_eq!(first.name, BOOK_SCENE_END);
    let second = relay
        .tick(gate.is_clear(), 16)
        .expect("the dropped copy holds nothing up");
    assert_eq!(second.name, BOOK_TALK_02);
}

#[test]
fn a_fill_in_copy_waits_behind_a_call_of_another_path() {
    let mut relay = FillInRelay::new();
    let mut gate = RelayCallGate::new();
    gate.submit(call(WAVE_END), Path::EndQueue, 0);
    relay.arrive(
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        FillInReach::CannotReachHere,
        0,
    );
    assert_eq!(relay.tick(gate.is_clear(), 0), None);
    let due = gate.tick(SlotState::Idle, false, 0, never_modal).unwrap();
    gate.queued(&due.call.function, 0x48, 0);
    assert_eq!(relay.tick(gate.is_clear(), 16), None);
    gate.popped(0x48);
    gate.tick(SlotState::Idle, false, 32, never_modal);
    let copy = relay.tick(gate.is_clear(), 32).unwrap();
    gate.submit(call(&copy.name), Path::FillIn, 32);
    assert_eq!(
        gate.tick(SlotState::Idle, false, 48, never_modal)
            .unwrap()
            .then,
        Path::FillIn
    );
}

#[test]
fn nothing_goes_while_the_fight_stream_has_its_own_call_queued() {
    let mut gate = RelayCallGate::new();
    gate.submit(call(BOOK_SCENE_END), Path::MessageHandler, 0);
    assert_eq!(gate.tick(SlotState::Idle, true, 0, never_modal), None);
    assert!(gate.tick(SlotState::Idle, false, 16, never_modal).is_some());
}

#[test]
fn a_queued_call_that_never_pops_frees_the_gate_with_one_notice() {
    let mut gate = RelayCallGate::new();
    gate.submit(call(BOOK_SCENE_END), Path::MessageHandler, 0);
    gate.submit(call(BOOK_TALK_02), Path::FillIn, 0);
    gate.tick(SlotState::Idle, false, 0, never_modal).unwrap();
    gate.queued(BOOK_SCENE_END, 0x48, 0);
    assert_eq!(
        gate.tick(
            SlotState::Idle,
            false,
            RELAY_CALL_POP_EXPECTED_MS,
            never_modal
        ),
        None
    );
    assert!(gate.take_notices().is_empty());
    let next = gate
        .tick(
            SlotState::Idle,
            false,
            RELAY_CALL_POP_EXPECTED_MS + 1,
            never_modal,
        )
        .unwrap();
    assert_eq!(next.call.function, BOOK_TALK_02);
    assert_eq!(
        gate.take_notices(),
        [RelayCallNotice::QueuedCallNeverPopped {
            name: BOOK_SCENE_END.to_owned(),
            waited_ms: RELAY_CALL_POP_EXPECTED_MS + 1
        }]
    );
}

#[test]
fn a_cancelled_call_frees_the_gate_and_a_restart_forgets_the_one_in_flight() {
    let mut gate = RelayCallGate::new();
    gate.submit(call(BOOK_SCENE_END), Path::EndQueue, 0);
    gate.tick(SlotState::Idle, false, 0, never_modal).unwrap();
    gate.queued(BOOK_SCENE_END, 0x48, 0);
    assert!(!gate.cancelled(0x900));
    assert!(gate.cancelled(0x48));
    assert!(gate.is_clear());
    gate.submit(call(BOOK_TALK_02), Path::EndQueue, 10);
    gate.tick(SlotState::Idle, false, 10, never_modal).unwrap();
    gate.queued(BOOK_TALK_02, 0x48, 10);
    assert_eq!(gate.forget_in_flight().as_deref(), Some(BOOK_TALK_02));
    assert_eq!(gate.in_flight(), &RelayCallFlight::Idle);
}

#[test]
fn a_held_slot_waits_unless_the_call_shows_modal_ui_which_closes_the_menu() {
    let mut gate = RelayCallGate::new();
    gate.submit(call(BOOK_SCENE_END), Path::MessageHandler, 0);
    assert_eq!(
        gate.tick(SlotState::SteppingHeld, false, 0, never_modal),
        None
    );
    assert_eq!(gate.tick(SlotState::Busy, false, 0, |_| true), None);
    assert!(
        gate.tick(SlotState::SteppingHeld, false, 0, |_| true)
            .is_some()
    );
}

#[test]
fn a_phase_change_drops_the_old_phases_waiting_calls() {
    let mut gate = RelayCallGate::new();
    gate.submit(call(BOOK_SCENE_END), Path::MessageHandler, 0);
    gate.submit(
        StoryCall {
            phase: STONE_FIELD_PHASE + 1,
            ..call(BOOK_TALK_02)
        },
        Path::FillIn,
        0,
    );
    assert_eq!(gate.enter_phase(Some(STONE_FIELD_PHASE + 1)), 1);
    assert_eq!(gate.waiting_len(), 1);
    assert_eq!(gate.clear(), 1);
    assert!(gate.is_clear());
}
