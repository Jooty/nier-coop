use std::collections::VecDeque;

use rc_game::fight_sync::{
    Arrival, BASE_PLAYER_LOCAL_HANDLERS, GuestTape, HostTape, InFlight, PopSource, PopVerdict,
    Replayer, SlotState, StreamEntry, StreamRecorder, TapeValue,
};
use rc_proto::StoryCallArg;

const STONE_FIELD_PHASE: u16 = 23;
const ATTACK_COUNT: &str = "AttackCount_Talk";
const TALK02_END: &str = "EID_0200_a0030g_Talk02_End";
const TALK03_END: &str = "EID_0200_a0030g_Talk03_End";
const TALK04_END: &str = "EID_0200_a0030g_Talk04_End";
const KILL_COUNT_END: &str = "EID_0200_a0037g_02_End";
const KILL_COUNT_MESSAGE: i32 = 212;
const BATTLE_PHASE_MESSAGE: i32 = 241;

// The script's own state, as c_A_STONE_FIELD_02 keeps it on each game. The hit counter only
// counts hits while no book talk is open; this reproduces where the run31 host log shows each
// talk end (after hits 4, 12 and 17) and the final hit 18.
#[derive(Default)]
struct StoneFieldScript {
    hit_count: u32,
    talk_open: bool,
    count_kill_enemy: u32,
    count_text: u32,
}

impl StoneFieldScript {
    fn run(&mut self, name: &str) -> Vec<String> {
        let mut ran = vec![name.to_owned()];
        let mut nested = |call: &str| ran.push(call.to_owned());
        match name {
            ATTACK_COUNT if !self.talk_open => {
                self.hit_count += 1;
                match self.hit_count {
                    1 => nested("EID_0200_a0030g_Talk02"),
                    8 => nested("EID_0200_a0030g_Talk03"),
                    11 => nested("EID_0200_a0030g_Talk04"),
                    12 => {
                        nested("EID_0200_a0030g_End");
                        nested("EID_0200_a0032_Start");
                    }
                    _ => {}
                }
                if matches!(self.hit_count, 1 | 8 | 11) {
                    self.talk_open = true;
                }
            }
            TALK02_END | TALK03_END | TALK04_END => self.talk_open = false,
            KILL_COUNT_END => {
                self.count_kill_enemy += 1;
                if self.count_kill_enemy >= 3 {
                    nested("EID_0200_a0037g_02");
                }
            }
            "CB_BossCA_01" => {
                nested("EID_0200_a0055g_End");
                nested("EID_0200_a0060_Start");
            }
            "EID_0200_a0060_End" => {
                nested("InitBattlePhase(2, CB_BossCA_02)");
                nested("EID_0200_a0065g_Start");
            }
            "CB_BossCA_02" => nested("EID_0200_a0065g_End"),
            "EID_0200_a0070_End" => {
                nested("InitBattlePhase(3, CB_BossCA_03)");
                nested("EID_0200_a0075g_Start");
            }
            "CB_BossCA_03" => nested("EID_0200_a0075g_End"),
            "Talk_Text_03_SHOT" | "Talk_Text_02_Damage" | "B_Lance_Talk" => {
                self.count_text += 1;
                let radio = format!("_SetRadio for Count_Text {}", self.count_text);
                nested(&radio);
            }
            _ => {}
        }
        ran
    }
}

fn handler_ticks(name: &str) -> u32 {
    match name {
        "EID_0200_a0030g_Talk02" | ATTACK_COUNT => 2,
        KILL_COUNT_END | "CB_BossCA_01" | "CB_BossCA_02" | "CB_BossCA_03" => 6,
        _ => 1,
    }
}

#[derive(Default)]
struct GuestEngine {
    own_fired: VecDeque<String>,
    queue: Vec<String>,
    running_ticks: u32,
    stepping_held: bool,
    most_queued: usize,
    replayed: Vec<String>,
    ran_player_local: Vec<String>,
    held_own_pops: Vec<String>,
}

impl GuestEngine {
    fn step(&mut self, replayer: &mut Replayer, script: &mut StoneFieldScript) {
        if self.stepping_held {
            assert_eq!(replayer.tick(SlotState::SteppingHeld), None);
            return;
        }
        self.running_ticks = self.running_ticks.saturating_sub(1);
        if self.running_ticks == 0 {
            let popped = if let Some(name) = self.own_fired.pop_front() {
                Some((name, PopSource::Fired))
            } else if self.queue.is_empty() {
                None
            } else {
                Some((self.queue.remove(0), PopSource::Queued))
            };
            if let Some((name, source)) = popped {
                match replayer.pop(&name, source) {
                    PopVerdict::RunReplayed { .. } => {
                        self.replayed.extend(script.run(&name));
                        self.running_ticks = handler_ticks(&name);
                    }
                    PopVerdict::RunPlayerLocal => {
                        self.ran_player_local.push(name);
                        self.running_ticks = 1;
                    }
                    PopVerdict::Hold => self.held_own_pops.push(name),
                }
            }
        }
        let slot = if self.running_ticks > 0 {
            SlotState::Busy
        } else {
            SlotState::Idle
        };
        if let Some(entry) = replayer.tick(slot) {
            self.queue.push(entry.name);
            self.most_queued = self.most_queued.max(self.queue.len());
        }
    }
}

struct HostRun {
    entries: Vec<StreamEntry>,
    ran: Vec<String>,
}

fn host_runs(pops: &[(&str, Option<i32>)]) -> HostRun {
    let mut recorder = StreamRecorder::new(STONE_FIELD_PHASE);
    let mut script = StoneFieldScript::default();
    let mut run = HostRun {
        entries: Vec::new(),
        ran: Vec::new(),
    };
    for (name, message) in pops {
        let args = message.map(StoryCallArg::Int).into_iter().collect();
        run.entries.push(recorder.record(name, args));
        run.ran.extend(script.run(name));
    }
    run
}

fn run31_book_pops() -> Vec<(&'static str, Option<i32>)> {
    // Host pops in the order of 2026-11-08-run31 game-host.log lines 1566-1776.
    let hits = |count: usize| std::iter::repeat_n((ATTACK_COUNT, None), count);
    hits(4)
        .chain([(TALK02_END, None)])
        .chain(hits(8))
        .chain([(TALK03_END, None)])
        .chain(hits(5))
        .chain([(TALK04_END, None)])
        .chain(hits(1))
        .chain([("EID_0200_a0032_End", None)])
        .collect()
}

fn run32_fight_pops() -> Vec<(&'static str, Option<i32>)> {
    // Host pops of 2026-11-08-run32 game-host.log lines 4306-11465; the run ended before
    // CB_BossCA_03, so its pop comes from `_SetMessageCallback('CB_BossCA_03',1,0,241)`.
    vec![
        (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
        (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
        (KILL_COUNT_END, Some(KILL_COUNT_MESSAGE)),
        ("Talk_Text_01_CA_END", None),
        ("CB_BossCA_01", Some(BATTLE_PHASE_MESSAGE)),
        ("EID_0200_a0060_Event", None),
        ("EID_0200_a0060_End", None),
        ("Talk_Text_03_SHOT", None),
        ("Talk_Text_02_Damage", None),
        ("B_Lance_Talk", None),
        ("CA_Generation_02", None),
        ("CB_BossCA_02", Some(BATTLE_PHASE_MESSAGE)),
        ("EID_0200_a0070_Start", None),
        ("EID_0200_a0050_Start", None),
        ("EID_0200_a0070_End", None),
        ("CB_BossCA_03", Some(BATTLE_PHASE_MESSAGE)),
    ]
}

fn replay_on_guest(
    host: &HostRun,
    entries_per_tick: usize,
    own_fired: &[(usize, &str)],
    held_ticks: std::ops::Range<usize>,
) -> GuestEngine {
    let mut replayer = Replayer::new(Vec::<String>::new());
    let mut script = StoneFieldScript::default();
    let mut guest = GuestEngine::default();
    let mut arriving = host.entries.iter().cloned().peekable();
    for tick in 0..10_000 {
        for _ in 0..entries_per_tick {
            if let Some(entry) = arriving.next() {
                assert_eq!(replayer.receive(entry), Arrival::Pending);
            }
        }
        for (at, name) in own_fired {
            if *at == tick {
                guest.own_fired.push_back((*name).to_owned());
            }
        }
        guest.stepping_held = held_ticks.contains(&tick);
        guest.step(&mut replayer, &mut script);
        let all_ran = arriving.peek().is_none()
            && replayer.pending_len() == 0
            && replayer.in_flight() == InFlight::Idle
            && guest.running_ticks == 0;
        if tick > held_ticks.end && all_ran {
            break;
        }
    }
    assert_eq!(replayer.next_seq(), host.entries.len() as u64);
    assert_eq!(replayer.in_flight(), InFlight::Idle);
    guest
}

#[test]
fn run31_book_host_sequence_reaches_the_end_and_the_next_start_nested() {
    let host = host_runs(&run31_book_pops());
    let end_at = host
        .ran
        .iter()
        .position(|call| call == "EID_0200_a0030g_End");
    let ends: Vec<&String> = host
        .ran
        .iter()
        .filter(|call| call.ends_with("_End"))
        .collect();
    assert_eq!(
        ends,
        [
            TALK02_END,
            TALK03_END,
            TALK04_END,
            "EID_0200_a0030g_End",
            "EID_0200_a0032_End"
        ]
    );
    let end_at = end_at.expect("the 12th counted hit runs the end");
    assert_eq!(host.ran[end_at - 1], ATTACK_COUNT);
    assert_eq!(host.ran[end_at + 1], "EID_0200_a0032_Start");
    assert_eq!(
        host.ran.iter().filter(|call| *call == ATTACK_COUNT).count(),
        18
    );
    assert_eq!(
        host.entries
            .iter()
            .map(|entry| entry.seq)
            .collect::<Vec<_>>(),
        (0..host.entries.len() as u64).collect::<Vec<_>>()
    );
}

#[test]
fn run31_book_replays_on_the_guest_in_the_host_order_with_nested_calls() {
    let host = host_runs(&run31_book_pops());
    for entries_per_tick in [1, 3, 100] {
        let guest = replay_on_guest(&host, entries_per_tick, &[], 0..0);
        assert_eq!(guest.replayed, host.ran);
        assert_eq!(guest.most_queued, 1);
    }
}

#[test]
fn run31_guest_own_hits_and_talk_ends_are_held_and_player_local_ones_run() {
    let host = host_runs(&run31_book_pops());
    let own = [
        (3, ATTACK_COUNT),
        (9, TALK02_END),
        (20, ATTACK_COUNT),
        (21, BASE_PLAYER_LOCAL_HANDLERS[1]),
        (40, "EID_0200_a0032_End"),
    ];
    let guest = replay_on_guest(&host, 1, &own, 0..0);
    assert_eq!(guest.replayed, host.ran);
    assert_eq!(
        guest.held_own_pops,
        [ATTACK_COUNT, TALK02_END, ATTACK_COUNT, "EID_0200_a0032_End"]
    );
    assert_eq!(guest.ran_player_local, [BASE_PLAYER_LOCAL_HANDLERS[1]]);
    assert_eq!(guest.most_queued, 1);
}

#[test]
fn run32_fight_replays_kill_count_and_battle_phases_in_the_host_order() {
    let host = host_runs(&run32_fight_pops());
    let scene_at = host
        .ran
        .iter()
        .position(|call| call == "EID_0200_a0037g_02");
    let third_kill = host
        .ran
        .iter()
        .enumerate()
        .filter(|(_, call)| *call == KILL_COUNT_END)
        .nth(2)
        .map(|(at, _)| at);
    assert_eq!(scene_at, third_kill.map(|at| at + 1));
    assert_eq!(
        host.entries
            .iter()
            .filter(|entry| entry.args == [StoryCallArg::Int(BATTLE_PHASE_MESSAGE)])
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["CB_BossCA_01", "CB_BossCA_02", "CB_BossCA_03"]
    );
    for entries_per_tick in [1, 2, 100] {
        let own = [(1, KILL_COUNT_END), (5, "CB_BossCA_01")];
        let guest = replay_on_guest(&host, entries_per_tick, &own, 0..0);
        assert_eq!(guest.replayed, host.ran);
        assert_eq!(guest.held_own_pops, [KILL_COUNT_END, "CB_BossCA_01"]);
        assert_eq!(guest.most_queued, 1);
    }
}

#[test]
fn a_menu_stretch_on_the_guest_waits_and_drops_nothing() {
    let host = host_runs(&run32_fight_pops());
    let guest = replay_on_guest(&host, 1, &[], 4..60);
    assert_eq!(guest.replayed, host.ran);
    assert_eq!(guest.most_queued, 1);
}

#[test]
fn nothing_is_released_while_stepping_is_held_or_the_slot_is_busy() {
    let host = host_runs(&run32_fight_pops());
    let mut replayer = Replayer::new(Vec::<String>::new());
    for entry in host.entries.iter().cloned() {
        replayer.receive(entry);
    }
    assert_eq!(replayer.tick(SlotState::SteppingHeld), None);
    assert_eq!(replayer.tick(SlotState::Busy), None);
    let first = replayer.tick(SlotState::Idle).expect("seq 0 is released");
    assert_eq!(first.seq, 0);
    assert_eq!(replayer.tick(SlotState::Idle), None);
    assert_eq!(
        replayer.pop(&first.name, PopSource::Queued),
        PopVerdict::RunReplayed { seq: 0 }
    );
    assert_eq!(replayer.tick(SlotState::Busy), None);
    assert_eq!(
        replayer.tick(SlotState::Idle).map(|entry| entry.seq),
        Some(1)
    );
}

#[test]
fn entries_are_released_by_seq_whatever_order_they_arrive_in() {
    let host = host_runs(&run32_fight_pops());
    let mut replayer = Replayer::new(Vec::<String>::new());
    let mut reversed = host.entries.clone();
    reversed.reverse();
    for entry in reversed {
        assert_eq!(replayer.receive(entry), Arrival::Pending);
    }
    let mut released = Vec::new();
    while let Some(entry) = replayer.tick(SlotState::Idle) {
        assert_eq!(
            replayer.pop(&entry.name, PopSource::Queued),
            PopVerdict::RunReplayed { seq: entry.seq }
        );
        released.push(entry);
    }
    assert_eq!(released, host.entries);
}

#[test]
fn a_repeated_entry_is_a_duplicate_and_runs_once() {
    let host = host_runs(&run31_book_pops());
    let mut replayer = Replayer::new(Vec::<String>::new());
    assert_eq!(replayer.receive(host.entries[0].clone()), Arrival::Pending);
    assert_eq!(
        replayer.receive(host.entries[0].clone()),
        Arrival::Duplicate
    );
    let first = replayer.tick(SlotState::Idle).expect("seq 0 is released");
    assert_eq!(
        replayer.receive(host.entries[0].clone()),
        Arrival::Duplicate
    );
    replayer.pop(&first.name, PopSource::Queued);
    replayer.tick(SlotState::Idle);
    assert_eq!(
        replayer.receive(host.entries[0].clone()),
        Arrival::Duplicate
    );
    assert_eq!(replayer.next_seq(), 1);
}

#[test]
fn a_restart_epoch_drops_stale_entries() {
    let mut recorder = StreamRecorder::new(STONE_FIELD_PHASE);
    let mut replayer = Replayer::new(Vec::<String>::new());
    let old: Vec<StreamEntry> = (0..4)
        .map(|_| recorder.record(KILL_COUNT_END, vec![StoryCallArg::Int(KILL_COUNT_MESSAGE)]))
        .collect();
    for entry in old.iter().skip(1).cloned() {
        replayer.receive(entry);
    }
    assert_eq!(replayer.pending_len(), 3);
    assert_eq!(recorder.restart(), 1);
    let fresh = recorder.record(
        "CB_BossCA_01",
        vec![StoryCallArg::Int(BATTLE_PHASE_MESSAGE)],
    );
    assert_eq!((fresh.epoch, fresh.seq), (1, 0));
    assert_eq!(replayer.receive(fresh.clone()), Arrival::Pending);
    assert_eq!(replayer.pending_len(), 1);
    assert_eq!(
        replayer.receive(old[0].clone()),
        Arrival::Stale { current_epoch: 1 }
    );
    assert_eq!(replayer.dropped_stale(), 4);
    assert_eq!(replayer.tick(SlotState::Idle), Some(fresh));
}

#[test]
fn a_restart_while_an_entry_is_queued_forgets_it() {
    let mut recorder = StreamRecorder::new(STONE_FIELD_PHASE);
    let mut replayer = Replayer::new(Vec::<String>::new());
    replayer.receive(recorder.record(KILL_COUNT_END, Vec::new()));
    assert!(replayer.tick(SlotState::Idle).is_some());
    recorder.restart();
    let fresh = recorder.record("CB_BossCA_01", Vec::new());
    replayer.receive(fresh.clone());
    assert_eq!(replayer.in_flight(), InFlight::Idle);
    assert_eq!(
        replayer.pop(KILL_COUNT_END, PopSource::Queued),
        PopVerdict::Hold
    );
    assert_eq!(replayer.tick(SlotState::Idle), Some(fresh));
}

#[test]
fn a_new_phase_starts_a_new_epoch_at_seq_zero() {
    let mut recorder = StreamRecorder::new(STONE_FIELD_PHASE);
    recorder.record(ATTACK_COUNT, Vec::new());
    assert_eq!(recorder.enter_phase(24), 1);
    let entry = recorder.record(ATTACK_COUNT, Vec::new());
    assert_eq!((entry.phase, entry.epoch, entry.seq), (24, 1, 0));
}

#[test]
fn a_streamed_player_local_call_is_skipped_on_the_guest() {
    let mut recorder = StreamRecorder::new(STONE_FIELD_PHASE);
    let mut replayer = Replayer::new(["Talk_Text_01_CA_END"]);
    replayer.receive(recorder.record(BASE_PLAYER_LOCAL_HANDLERS[0], Vec::new()));
    replayer.receive(recorder.record("Talk_Text_01_CA_END", Vec::new()));
    let fight = recorder.record("CB_BossCA_01", Vec::new());
    replayer.receive(fight.clone());
    assert_eq!(replayer.tick(SlotState::Idle), Some(fight));
    assert_eq!(
        replayer.pop("Talk_Text_01_CA_END", PopSource::Fired),
        PopVerdict::RunPlayerLocal
    );
}

#[test]
fn the_guest_reads_the_host_taped_values_by_seq_native_and_call_index() {
    let mut host = HostTape::new();
    host.begin(7);
    let taped: Vec<_> = [
        ("_GetActorLife", TapeValue::Int(120)),
        ("_GetActorLife", TapeValue::Int(80)),
        ("_GetRandI", TapeValue::Int(3)),
    ]
    .into_iter()
    .filter_map(|(native, value)| host.record(native, value))
    .collect();
    host.end();
    assert_eq!(taped.len(), 3);
    let mut guest = GuestTape::new();
    guest.load(taped);
    guest.begin(7);
    let first = guest.read("_GetActorLife", TapeValue::Int(500));
    let rand = guest.read("_GetRandI", TapeValue::Int(0));
    let second = guest.read("_GetActorLife", TapeValue::Int(500));
    assert_eq!(
        [first.value, second.value, rand.value],
        [TapeValue::Int(120), TapeValue::Int(80), TapeValue::Int(3)]
    );
    assert!(
        [first.log, second.log, rand.log]
            .iter()
            .all(Option::is_none)
    );
}

#[test]
fn a_missing_or_mismatched_tape_value_uses_the_local_value_and_logs_once() {
    let mut host = HostTape::new();
    host.begin(2);
    let taped = host.record("_GetActorLifeRate", TapeValue::Int(1));
    host.end();
    let mut guest = GuestTape::new();
    guest.load(taped.into_iter().collect());
    guest.begin(2);
    let mismatch = guest.read("_GetActorLifeRate", TapeValue::Float(0.5));
    assert_eq!(mismatch.value, TapeValue::Float(0.5));
    let line = mismatch.log.expect("a mismatch logs");
    assert!(line.contains("_GetActorLifeRate") && line.contains("seq 2"));
    let missing = guest.read("_GetActorLifeRate", TapeValue::Float(0.25));
    assert_eq!(missing.value, TapeValue::Float(0.25));
    assert_eq!(missing.log, None);
    let other = guest.read("_GetRandI", TapeValue::Int(4));
    assert_eq!(other.value, TapeValue::Int(4));
    assert!(other.log.is_some());
}

#[test]
fn reads_outside_a_replayed_handler_are_local_and_silent() {
    let mut host = HostTape::new();
    assert_eq!(host.record("_GetRandI", TapeValue::Int(9)), None);
    let mut guest = GuestTape::new();
    let read = guest.read("_GetRandI", TapeValue::Int(2));
    assert_eq!((read.value, read.log), (TapeValue::Int(2), None));
    guest.begin(0);
    guest.end();
    let after = guest.read("_GetRandI", TapeValue::Int(5));
    assert_eq!((after.value, after.log), (TapeValue::Int(5), None));
}
