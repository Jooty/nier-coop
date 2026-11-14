use automata_game::enemies::{
    CountVerdict, GENERATOR_ACTION_ID_OFFSET, GENERATOR_COUNTS_MAY_DIFFER_FOR_MS,
    GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET, GENERATOR_HEADER_LEN, GENERATOR_MOST_ENEMIES_ALIVE_OFFSET,
    GENERATOR_REST_TIME_OFFSET, GENERATOR_SLOT_STATE_ENEMY_MADE,
    GENERATOR_SLOT_STATE_ENEMY_NOT_MADE, GENERATOR_SLOT_STATE_FREED, GENERATOR_SLOT_STATE_NEW,
    GENERATOR_SPAWNED_COUNT_OFFSET, GENERATOR_STATE_HASH_FINISHED,
    GENERATOR_STATE_HASH_NOT_STARTED, GENERATOR_STATE_HASH_OFFSET, GENERATOR_STATE_HASH_SPAWNING,
    GENERATOR_STATE_HASH_WAITING, GeneratorCountReading, GeneratorCountWatch, GeneratorHeader,
    GeneratorPhase, GeneratorRunCounts, GeneratorSlot, GeneratorState, GeneratorStates, HostSpawns,
    generator_dead_count, generator_header, generator_takes_the_hosts_run_counts_line,
    host_run_counts_to_write,
};

const A_GENERATOR: u32 = 0x5A01_77C2;
const ANOTHER_GENERATOR: u32 = 0x1B22_0C9E;
const SLOT_STATE_LOADING: i32 = 1;
const SLOT_STATE_READY: i32 = 2;

fn header(
    state_hash: u32,
    spawned: u64,
    run: i32,
    most_alive: i32,
    rest_s: f32,
) -> GeneratorHeader {
    let mut bytes = vec![0_u8; GENERATOR_HEADER_LEN];
    let mut put = |offset: usize, value: &[u8]| {
        bytes[offset..offset + value.len()].copy_from_slice(value);
    };
    put(GENERATOR_ACTION_ID_OFFSET, &A_GENERATOR.to_le_bytes());
    put(GENERATOR_STATE_HASH_OFFSET, &state_hash.to_le_bytes());
    put(GENERATOR_SPAWNED_COUNT_OFFSET, &spawned.to_le_bytes());
    put(GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET, &run.to_le_bytes());
    put(
        GENERATOR_MOST_ENEMIES_ALIVE_OFFSET,
        &most_alive.to_le_bytes(),
    );
    put(GENERATOR_REST_TIME_OFFSET, &rest_s.to_le_bytes());
    generator_header(&bytes).expect("a whole header")
}

fn slot(state: i32) -> GeneratorSlot {
    GeneratorSlot {
        state,
        set_index: 0,
        handle: 0x40_0001,
        position: [0.0; 4],
        rotation: [0.0; 4],
    }
}

fn counts(run: i16, dead: i16) -> GeneratorRunCounts {
    GeneratorRunCounts {
        enemies_of_one_run: run,
        most_enemies_alive: 3,
        rest_time_bits: 2.5_f32.to_bits(),
        dead,
    }
}

fn hosts(
    phase: GeneratorPhase,
    spawned: u32,
    run_counts: Option<GeneratorRunCounts>,
) -> GeneratorState {
    GeneratorState {
        generator: A_GENERATOR,
        phase,
        spawned,
        counts: run_counts,
    }
}

fn reading(
    phase: GeneratorPhase,
    spawned: u64,
    run_counts: Option<GeneratorRunCounts>,
) -> GeneratorCountReading {
    GeneratorCountReading {
        generator: A_GENERATOR,
        phase,
        spawned,
        counts: run_counts,
    }
}

#[test]
fn state_hash_zero_is_the_known_state_not_started() {
    let phase = GeneratorPhase::of_state_hash(GENERATOR_STATE_HASH_NOT_STARTED);
    assert_eq!(phase, GeneratorPhase::NotStarted);
    assert!(phase.is_not_started());
    assert_eq!(phase.label(), "not started");
    assert!(!phase.is_after_a_run());
    assert!(!GeneratorPhase::Waiting.is_not_started());
    assert!(!GeneratorPhase::Other.is_not_started());
}

#[test]
fn a_generator_not_started_is_never_read_for_counts() {
    let not_started = header(
        GENERATOR_STATE_HASH_NOT_STARTED,
        0x7FFF,
        0x1234,
        0x99,
        f32::NAN,
    );
    assert_eq!(not_started.phase(), GeneratorPhase::NotStarted);
    assert_eq!(GeneratorRunCounts::of_generator(&not_started, &[]), None);
    assert_eq!(
        GeneratorCountReading::of_generator(&not_started, &[]).counts,
        None
    );
}

#[test]
fn a_started_generator_reports_its_rolled_counts_and_its_dead_count() {
    let spawning = header(GENERATOR_STATE_HASH_SPAWNING, 4, 6, 3, 2.5);
    let slots = [
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE),
        slot(GENERATOR_SLOT_STATE_FREED),
    ];
    assert_eq!(
        GeneratorRunCounts::of_generator(&spawning, &slots),
        Some(GeneratorRunCounts {
            enemies_of_one_run: 6,
            most_enemies_alive: 3,
            rest_time_bits: 2.5_f32.to_bits(),
            dead: 3,
        })
    );
    let too_big = header(GENERATOR_STATE_HASH_SPAWNING, 4, 0x1_0000, 3, 2.5);
    assert_eq!(GeneratorRunCounts::of_generator(&too_big, &slots), None);
}

#[test]
fn the_dead_count_is_the_made_count_minus_the_slots_still_loading_or_alive() {
    let slots = [
        slot(GENERATOR_SLOT_STATE_NEW),
        slot(SLOT_STATE_LOADING),
        slot(SLOT_STATE_READY),
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE),
        slot(GENERATOR_SLOT_STATE_FREED),
        slot(GENERATOR_SLOT_STATE_ENEMY_NOT_MADE),
    ];
    assert_eq!(
        generator_dead_count(GeneratorPhase::Spawning, 7, 9, &slots),
        3
    );
    assert_eq!(
        generator_dead_count(GeneratorPhase::SwitchedOff, 4, 9, &slots),
        0
    );
    assert_eq!(generator_dead_count(GeneratorPhase::Waiting, 2, 9, &[]), 2);
}

#[test]
fn a_forced_defeat_counts_as_dead_once_its_slot_is_freed() {
    let alive = [
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE),
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE),
    ];
    let one_defeated = [
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE),
        slot(GENERATOR_SLOT_STATE_FREED),
    ];
    assert_eq!(
        generator_dead_count(GeneratorPhase::Spawning, 2, 2, &alive),
        0
    );
    assert_eq!(
        generator_dead_count(GeneratorPhase::Spawning, 2, 2, &one_defeated),
        1
    );
}

#[test]
fn a_finished_generator_counts_its_whole_run_as_dead() {
    assert_eq!(
        generator_dead_count(
            GeneratorPhase::Finished,
            0,
            5,
            &[slot(GENERATOR_SLOT_STATE_ENEMY_MADE)]
        ),
        5
    );
    let finished = header(GENERATOR_STATE_HASH_FINISHED, 0, 5, 2, 0.0);
    assert_eq!(
        GeneratorRunCounts::of_generator(&finished, &[]).map(|counts| counts.dead),
        Some(5)
    );
}

#[test]
fn the_hosts_run_counts_are_written_only_when_both_copies_have_started_the_generator() {
    let sent = hosts(GeneratorPhase::Spawning, 2, Some(counts(6, 1)));
    assert_eq!(
        host_run_counts_to_write(GeneratorPhase::Spawning, Some(&sent)),
        Some(counts(6, 1))
    );
    assert_eq!(
        host_run_counts_to_write(GeneratorPhase::Waiting, Some(&sent)),
        Some(counts(6, 1))
    );
    assert_eq!(
        host_run_counts_to_write(GeneratorPhase::NotStarted, Some(&sent)),
        None
    );
    let not_started_on_the_host = hosts(GeneratorPhase::NotStarted, 0, Some(counts(6, 1)));
    assert_eq!(
        host_run_counts_to_write(GeneratorPhase::Spawning, Some(&not_started_on_the_host)),
        None
    );
}

#[test]
fn without_the_hosts_counts_the_guest_keeps_its_own_roll() {
    assert_eq!(
        host_run_counts_to_write(GeneratorPhase::Spawning, None),
        None
    );
    let without_counts = hosts(GeneratorPhase::Spawning, 2, None);
    assert_eq!(
        host_run_counts_to_write(GeneratorPhase::Spawning, Some(&without_counts)),
        None
    );
    let unusable = hosts(
        GeneratorPhase::Spawning,
        2,
        Some(GeneratorRunCounts {
            rest_time_bits: f32::INFINITY.to_bits(),
            ..counts(6, 1)
        }),
    );
    assert_eq!(
        host_run_counts_to_write(GeneratorPhase::Spawning, Some(&unusable)),
        None
    );
}

#[test]
fn the_guests_reading_is_the_same_when_state_made_run_and_dead_agree() {
    let host = hosts(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    let own = reading(
        GeneratorPhase::Spawning,
        3,
        Some(GeneratorRunCounts {
            most_enemies_alive: 9,
            ..counts(6, 1)
        }),
    );
    assert_eq!(own.verdict(Some(&host)), CountVerdict::Same);
    let host_without_counts = hosts(GeneratorPhase::Spawning, 3, None);
    assert_eq!(own.verdict(Some(&host_without_counts)), CountVerdict::Same);
}

#[test]
fn any_of_the_readings_a_script_takes_that_differs_is_a_difference() {
    let host = hosts(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    for own in [
        reading(GeneratorPhase::Finished, 3, Some(counts(6, 1))),
        reading(GeneratorPhase::Spawning, 2, Some(counts(6, 1))),
        reading(GeneratorPhase::Spawning, 3, Some(counts(5, 1))),
        reading(GeneratorPhase::Spawning, 3, Some(counts(6, 2))),
    ] {
        assert_eq!(own.verdict(Some(&host)), CountVerdict::Differ, "{own:?}");
    }
}

#[test]
fn a_generator_started_on_the_host_and_not_here_is_its_own_case() {
    let host = hosts(GeneratorPhase::Waiting, 0, Some(counts(6, 0)));
    let own = reading(GeneratorPhase::NotStarted, 0, None);
    let verdict = own.verdict(Some(&host));
    assert_eq!(verdict, CountVerdict::StartedOnlyOnTheHost);
    assert!(verdict.is_a_difference());
    assert!(verdict.label().contains("given no slot"));
    let not_started_there = hosts(GeneratorPhase::NotStarted, 0, None);
    assert_eq!(
        own.verdict(Some(&not_started_there)),
        CountVerdict::NeitherStarted
    );
    assert_eq!(own.verdict(None), CountVerdict::NeitherStarted);
    assert_eq!(
        reading(GeneratorPhase::Waiting, 0, Some(counts(6, 0))).verdict(None),
        CountVerdict::HostSaidNothing
    );
}

#[test]
fn a_change_gets_a_line_with_both_copies_counts() {
    let mut watch = GeneratorCountWatch::default();
    let host = hosts(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    let own = reading(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    assert!(watch.note(0, &[(own, Some(host))]).is_empty());
    let changed = reading(GeneratorPhase::Spawning, 3, Some(counts(6, 2)));
    let lines = watch.note(1_000, &[(changed, Some(host))]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("counts of generator 0x5A0177C2"));
    assert!(lines[0].contains("the host's spawning, 3 made, run of 6"));
    assert!(lines[0].contains("1 dead"));
    assert!(lines[0].contains("2 dead"));
    assert!(lines[0].contains("they differ"));
    assert!(watch.note(1_500, &[(changed, Some(host))]).is_empty());
}

#[test]
fn generators_started_on_neither_copy_stay_quiet() {
    let mut watch = GeneratorCountWatch::default();
    let own = reading(GeneratorPhase::NotStarted, 0, None);
    let host = hosts(GeneratorPhase::NotStarted, 0, None);
    for now_ms in [0, 1_000, 5_000] {
        assert!(watch.note(now_ms, &[(own, Some(host))]).is_empty());
    }
}

#[test]
fn a_difference_lasting_longer_than_its_bound_gets_a_line_a_second() {
    let mut watch = GeneratorCountWatch::default();
    let host = hosts(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    let own = reading(GeneratorPhase::Spawning, 2, Some(counts(6, 0)));
    assert_eq!(watch.note(0, &[(own, Some(host))]).len(), 1);
    assert!(watch.note(1_000, &[(own, Some(host))]).is_empty());
    assert!(
        watch
            .note(GENERATOR_COUNTS_MAY_DIFFER_FOR_MS, &[(own, Some(host))])
            .is_empty()
    );
    let late = GENERATOR_COUNTS_MAY_DIFFER_FOR_MS + 1;
    let lines = watch.note(late, &[(own, Some(host))]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("still differ after 2001 ms"));
    assert!(watch.note(late + 500, &[(own, Some(host))]).is_empty());
    assert_eq!(watch.note(late + 1_000, &[(own, Some(host))]).len(), 1);
}

#[test]
fn the_bound_starts_again_once_the_counts_agree() {
    let mut watch = GeneratorCountWatch::default();
    let host = hosts(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    let differs = reading(GeneratorPhase::Spawning, 2, Some(counts(6, 0)));
    let agrees = reading(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    watch.note(0, &[(differs, Some(host))]);
    watch.note(1_000, &[(agrees, Some(host))]);
    watch.note(2_000, &[(differs, Some(host))]);
    assert!(watch.note(4_000, &[(differs, Some(host))]).is_empty());
    assert_eq!(watch.note(4_001, &[(differs, Some(host))]).len(), 1);
}

#[test]
fn a_generator_started_only_on_the_host_is_named_at_once_and_while_it_lasts() {
    let mut watch = GeneratorCountWatch::default();
    let host = hosts(GeneratorPhase::Spawning, 2, Some(counts(6, 0)));
    let own = reading(GeneratorPhase::NotStarted, 0, None);
    let first = watch.note(0, &[(own, Some(host))]);
    assert_eq!(first.len(), 1);
    assert!(first[0].contains("here not started, 0 made, no run counts"));
    assert!(first[0].contains("this copy's script has not started it"));
    assert_eq!(watch.note(3_000, &[(own, Some(host))]).len(), 1);
}

#[test]
fn a_generator_no_longer_read_is_forgotten() {
    let mut watch = GeneratorCountWatch::default();
    let host = hosts(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    let own = reading(GeneratorPhase::Spawning, 2, Some(counts(6, 0)));
    watch.note(0, &[(own, Some(host))]);
    let other = GeneratorCountReading {
        generator: ANOTHER_GENERATOR,
        ..reading(GeneratorPhase::NotStarted, 0, None)
    };
    watch.note(1_000, &[(other, None)]);
    assert_eq!(watch.note(5_000, &[(own, Some(host))]).len(), 1);
    assert!(watch.note(5_001, &[(own, Some(host))]).is_empty());
}

#[test]
fn the_count_lines_stop_at_the_minutes_budget_and_the_next_minute_says_how_many_were_held_back() {
    let mut watch = GeneratorCountWatch::default();
    let host = hosts(GeneratorPhase::Spawning, 3, Some(counts(6, 1)));
    let mut all = Vec::new();
    for step in 0..400_u64 {
        let own = reading(GeneratorPhase::Spawning, step, Some(counts(6, 0)));
        all.extend(watch.note(step * 10, &[(own, Some(host))]));
    }
    assert_eq!(
        all.len(),
        automata_game::enemies::GENERATOR_COUNT_LINES_LOGGED_PER_MINUTE as usize
    );
    let own = reading(GeneratorPhase::Spawning, 9_999, Some(counts(6, 0)));
    let next = watch.note(61_000, &[(own, Some(host))]);
    assert!(next.len() >= 2);
    assert!(next[0].contains("more lines of this kind held back until the next minute"));
}

#[test]
fn the_write_line_names_the_hosts_counts_beside_the_guests_own_roll() {
    let own = header(GENERATOR_STATE_HASH_WAITING, 0, 4, 2, 1.0);
    let line = generator_takes_the_hosts_run_counts_line(&own, &counts(6, 0), Some(2));
    assert!(line.contains("generator 0x5A0177C2 takes the host's run counts after its own roll"));
    assert!(line.contains("run of 6 (its own 4)"));
    assert!(line.contains("at most 3 alive (its own 2)"));
    assert!(line.contains("rest 2.5 s (its own 1.0 s)"));
    assert!(line.contains("is the host's 2 outside the game's spawn decision"));
}

#[test]
fn a_states_message_with_run_counts_no_generator_can_hold_is_not_valid() {
    let good = GeneratorStates {
        states: vec![hosts(GeneratorPhase::Spawning, 2, Some(counts(6, 1)))],
    };
    assert!(good.is_valid());
    let bad = GeneratorStates {
        states: vec![hosts(GeneratorPhase::Spawning, 2, Some(counts(-1, 1)))],
    };
    assert!(!bad.is_valid());
}

#[test]
fn the_host_tells_the_guest_again_when_only_the_run_counts_change() {
    let mut host = HostSpawns::default();
    let first = hosts(GeneratorPhase::Spawning, 2, Some(counts(6, 0)));
    assert_eq!(host.states_to_tell(&[first]).len(), 1);
    assert!(host.states_to_tell(&[first]).is_empty());
    let one_dead = hosts(GeneratorPhase::Spawning, 2, Some(counts(6, 1)));
    assert_eq!(host.states_to_tell(&[one_dead])[0].states, vec![one_dead]);
}
