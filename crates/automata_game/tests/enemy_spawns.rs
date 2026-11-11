use automata_game::address::{
    GENERATOR_LEVEL_PICK_FUNCTION, GENERATOR_LIST_HEAD_VALUE, GENERATOR_SLOT_ADD_FUNCTION,
    GENERATOR_SLOT_FREE_FUNCTION, GENERATOR_SLOT_STEP_FUNCTION, GENERATOR_SPAWNER_UPDATE_FUNCTION,
};
use automata_game::enemies::{
    EnemyId, EnemySpawn, GENERATOR_ACTION_ID_OFFSET, GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET,
    GENERATOR_HEADER_LEN, GENERATOR_NEXT_POINTER_OFFSET, GENERATOR_SET_LEN,
    GENERATOR_SLOT_COUNT_OFFSET, GENERATOR_SLOT_LEN, GENERATOR_SLOT_STATE_ENEMY_MADE,
    GENERATOR_SLOT_STATE_ENEMY_NOT_MADE, GENERATOR_SLOT_STATE_FREED,
    GENERATOR_SLOTS_POINTER_OFFSET, GENERATOR_SPAWNED_COUNT_OFFSET, GENERATOR_STATE_HASH_OFFSET,
    GENERATOR_USES_THE_SECOND_SET_VALUES_OFFSET, GeneratorCensus, GeneratorHeader, GeneratorPhase,
    GeneratorSlot, GeneratorState, GuestGenerators, HostSpawns, ObservedSpawn, SlotRemoval,
    generator_header, generator_slots, generators_with_a_shared_id, new_slot_bytes,
    object_id_of_set,
};
use automata_game::exe::ExeVariant;

const A_GENERATOR: u32 = 0x5A01_77C2;
const ANOTHER_GENERATOR: u32 = 0x1B22_0C9E;
const A_SMALL_ENEMY: u32 = 0x2_0000;
const A_PLACE: [f32; 4] = [120.5, -3.25, 44.0, 1.0];
const ANOTHER_PLACE: [f32; 4] = [121.0, -3.25, 47.5, 1.0];
const A_FACING: [f32; 4] = [0.0, 1.5, 0.0, 1.0];

fn observed(generator: u32, handle: u32, position: [f32; 4]) -> ObservedSpawn {
    ObservedSpawn {
        generator,
        set_index: 2,
        object_id: A_SMALL_ENEMY,
        level: Some(14),
        uses_the_second_set_values: false,
        object_read_variant: 0,
        set_values: [0, 0, 0, 0],
        handle,
        position,
        rotation: A_FACING,
    }
}

fn spawn(serial: u32, position: [f32; 4]) -> EnemySpawn {
    EnemySpawn {
        generator: A_GENERATOR,
        serial,
        set_index: 2,
        object_id: A_SMALL_ENEMY,
        level: Some(14),
        uses_the_second_set_values: false,
        object_read_variant: 0,
        set_values: [0, 0, 0, 0],
        position,
        rotation: A_FACING,
    }
}

fn id(serial: u32) -> EnemyId {
    EnemyId::Generated {
        generator: A_GENERATOR,
        serial,
    }
}

fn slot(state: i32, handle: u32, position: [f32; 4]) -> GeneratorSlot {
    GeneratorSlot {
        state,
        set_index: 2,
        handle,
        position,
        rotation: A_FACING,
    }
}

fn header(action_id: u32, state_hash: u32) -> GeneratorHeader {
    let mut bytes = vec![0_u8; GENERATOR_HEADER_LEN];
    bytes[GENERATOR_ACTION_ID_OFFSET..GENERATOR_ACTION_ID_OFFSET + 4]
        .copy_from_slice(&action_id.to_le_bytes());
    bytes[GENERATOR_STATE_HASH_OFFSET..GENERATOR_STATE_HASH_OFFSET + 4]
        .copy_from_slice(&state_hash.to_le_bytes());
    generator_header(&bytes).expect("a whole header")
}

fn guest_with_a_made_enemy(handle: u32) -> GuestGenerators {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.note_added(id(1));
    guest.note_slots(
        A_GENERATOR,
        &[slot(GENERATOR_SLOT_STATE_ENEMY_MADE, handle, A_PLACE)],
    );
    guest
}

#[test]
fn the_generator_addresses_are_picked_per_exe() {
    for (address, main, compat) in [
        (GENERATOR_LIST_HEAD_VALUE, 0xFBF730, 0xFB3630),
        (GENERATOR_SPAWNER_UPDATE_FUNCTION, 0x359990, 0x353DD0),
        (GENERATOR_SLOT_STEP_FUNCTION, 0x359DC0, 0x354200),
        (GENERATOR_LEVEL_PICK_FUNCTION, 0x3504A0, 0x34A880),
        (GENERATOR_SLOT_FREE_FUNCTION, 0x34D9B0, 0x347D90),
        (GENERATOR_SLOT_ADD_FUNCTION, 0x34D3F0, 0x3477D0),
    ] {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}

#[test]
fn a_generator_header_is_read_from_the_games_bytes() {
    let mut bytes = vec![0_u8; GENERATOR_HEADER_LEN];
    bytes[GENERATOR_ACTION_ID_OFFSET..GENERATOR_ACTION_ID_OFFSET + 4]
        .copy_from_slice(&A_GENERATOR.to_le_bytes());
    bytes[GENERATOR_STATE_HASH_OFFSET..GENERATOR_STATE_HASH_OFFSET + 4]
        .copy_from_slice(&0xE361_D0B2_u32.to_le_bytes());
    bytes[GENERATOR_SPAWNED_COUNT_OFFSET..GENERATOR_SPAWNED_COUNT_OFFSET + 8]
        .copy_from_slice(&5_u64.to_le_bytes());
    bytes[GENERATOR_SLOTS_POINTER_OFFSET..GENERATOR_SLOTS_POINTER_OFFSET + 8]
        .copy_from_slice(&0x1_4000_2000_u64.to_le_bytes());
    bytes[GENERATOR_SLOT_COUNT_OFFSET..GENERATOR_SLOT_COUNT_OFFSET + 8]
        .copy_from_slice(&3_u64.to_le_bytes());
    bytes[GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET..GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET + 4]
        .copy_from_slice(&12_i32.to_le_bytes());
    bytes[GENERATOR_USES_THE_SECOND_SET_VALUES_OFFSET] = 1;
    bytes[GENERATOR_NEXT_POINTER_OFFSET..GENERATOR_NEXT_POINTER_OFFSET + 8]
        .copy_from_slice(&0x1_4000_9000_u64.to_le_bytes());
    let header = generator_header(&bytes).expect("a whole header");
    assert_eq!(header.action_id, A_GENERATOR);
    assert_eq!(header.phase(), GeneratorPhase::Spawning);
    assert_eq!(header.spawned, 5);
    assert_eq!(header.slots, 0x1_4000_2000);
    assert_eq!(header.slot_count, 3);
    assert_eq!(header.enemies_of_one_run, 12);
    assert!(header.uses_the_second_set_values);
    assert_eq!(header.next, 0x1_4000_9000);
    assert_eq!(generator_header(&bytes[..GENERATOR_HEADER_LEN - 1]), None);
}

#[test]
fn every_state_hash_of_a_generator_has_its_phase() {
    for (hash, phase) in [
        (0x4B7B_6E51, GeneratorPhase::Waiting),
        (0xE361_D0B2, GeneratorPhase::Spawning),
        (0xAF40_F7B2, GeneratorPhase::Resting),
        (0x26B8_0D1F, GeneratorPhase::Finished),
        (0xE742_EEB9, GeneratorPhase::SwitchedOff),
        (0x1234_5678, GeneratorPhase::Other),
    ] {
        assert_eq!(GeneratorPhase::of_state_hash(hash), phase);
    }
    assert!(GeneratorPhase::Resting.is_after_a_run());
    assert!(GeneratorPhase::Finished.is_after_a_run());
    assert!(!GeneratorPhase::Spawning.is_after_a_run());
    assert!(!GeneratorPhase::Waiting.is_after_a_run());
}

#[test]
fn a_new_slot_reads_back_as_the_slot_the_game_would_have_added() {
    let bytes = new_slot_bytes(2, &A_PLACE, &A_FACING);
    assert_eq!(bytes.len(), GENERATOR_SLOT_LEN);
    let slots = generator_slots(&bytes);
    assert_eq!(slots, vec![slot(0, 0, A_PLACE)]);
    assert!(slots[0].waits_for_its_enemy());
    assert!(!slots[0].holds_a_made_enemy());
    assert!(slots[0].was_made_for(2, &A_PLACE, &A_FACING));
    assert!(!slots[0].was_made_for(3, &A_PLACE, &A_FACING));
    assert!(!slots[0].was_made_for(2, &ANOTHER_PLACE, &A_FACING));
}

#[test]
fn a_cut_off_last_slot_is_left_out() {
    let mut bytes = new_slot_bytes(2, &A_PLACE, &A_FACING).to_vec();
    bytes
        .extend_from_slice(&new_slot_bytes(1, &ANOTHER_PLACE, &A_FACING)[..GENERATOR_SLOT_LEN - 1]);
    assert_eq!(generator_slots(&bytes).len(), 1);
}

#[test]
fn the_object_of_a_set_is_read_by_its_index() {
    let mut sets = vec![0_u8; GENERATOR_SET_LEN * 2];
    sets[GENERATOR_SET_LEN..GENERATOR_SET_LEN + 4].copy_from_slice(&A_SMALL_ENEMY.to_le_bytes());
    assert_eq!(object_id_of_set(&sets, 1), Some(A_SMALL_ENEMY));
    assert_eq!(object_id_of_set(&sets, 2), None);
}

#[test]
fn a_generator_id_two_live_generators_carry_names_none_of_them() {
    let generators = [
        header(A_GENERATOR, 0x4B7B_6E51),
        header(ANOTHER_GENERATOR, 0x4B7B_6E51),
        header(A_GENERATOR, 0xE361_D0B2),
        header(0, 0x4B7B_6E51),
    ];
    let shared = generators_with_a_shared_id(&generators);
    assert!(shared.contains(&A_GENERATOR));
    assert!(shared.contains(&0));
    assert!(!shared.contains(&ANOTHER_GENERATOR));
}

#[test]
fn the_generator_census_says_seen_then_a_new_phase_then_gone() {
    let mut census = GeneratorCensus::default();
    let seen = census.note(&[header(A_GENERATOR, 0x4B7B_6E51)]);
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0], "generators: 1 live, ids (hex) 5A0177C2");
    assert!(seen[1].starts_with("generators: seen generator 0x5A0177C2 (waiting"));
    assert!(census.note(&[header(A_GENERATOR, 0x4B7B_6E51)]).is_empty());
    let changed = census.note(&[header(A_GENERATOR, 0xE361_D0B2)]);
    assert_eq!(changed.len(), 1);
    assert!(changed[0].starts_with("generators: now generator 0x5A0177C2 (spawning"));
    let gone = census.note(&[]);
    assert_eq!(
        gone,
        vec!["generators: gone generator 0x5A0177C2".to_owned()]
    );
}

#[test]
fn the_host_gives_each_spawn_of_a_generator_the_next_serial() {
    let mut host = HostSpawns::default();
    let first = host.note(observed(A_GENERATOR, 0x100, A_PLACE));
    let second = host.note(observed(A_GENERATOR, 0x200, ANOTHER_PLACE));
    let other = host.note(observed(ANOTHER_GENERATOR, 0x300, A_PLACE));
    assert_eq!(first.id(), id(1));
    assert_eq!(second.id(), id(2));
    assert_eq!(
        other.id(),
        EnemyId::Generated {
            generator: ANOTHER_GENERATOR,
            serial: 1
        }
    );
    assert_eq!(first.level, Some(14));
    assert_eq!(first.position, A_PLACE);
    assert_eq!(host.living().len(), 3);
}

#[test]
fn a_serial_is_not_given_again_after_its_enemy_is_gone() {
    let mut host = HostSpawns::default();
    let first = host.note(observed(A_GENERATOR, 0x100, A_PLACE));
    assert!(host.note_gone(first.id()));
    assert!(!host.note_gone(first.id()));
    let next = host.note(observed(A_GENERATOR, 0x100, A_PLACE));
    assert_eq!(next.id(), id(2));
    assert_eq!(host.spawns_still_there(), vec![next]);
}

#[test]
fn the_host_tells_a_generator_state_once_until_it_changes() {
    let mut host = HostSpawns::default();
    let waiting = GeneratorState::untold(A_GENERATOR);
    assert!(host.states_to_tell(&[waiting]).is_empty());
    let spawning = GeneratorState {
        generator: A_GENERATOR,
        phase: GeneratorPhase::Spawning,
        spawned: 2,
    };
    let told = host.states_to_tell(&[spawning]);
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].states, vec![spawning]);
    assert!(host.states_to_tell(&[spawning]).is_empty());
    assert_eq!(host.states_to_tell(&[waiting])[0].states, vec![waiting]);
    host.states_to_tell(&[spawning]);
    host.forget_what_the_guest_was_told();
    assert_eq!(host.states_to_tell(&[spawning])[0].states, vec![spawning]);
}

#[test]
fn many_changed_generator_states_go_out_in_messages_of_sixteen() {
    let mut host = HostSpawns::default();
    let states: Vec<GeneratorState> = (1..=40)
        .map(|generator| GeneratorState {
            generator,
            phase: GeneratorPhase::Finished,
            spawned: 0,
        })
        .collect();
    let messages = host.states_to_tell(&states);
    assert_eq!(
        messages
            .iter()
            .map(|message| message.states.len())
            .collect::<Vec<_>>(),
        vec![16, 16, 8]
    );
    assert!(messages.iter().all(|message| message.is_valid()));
}

#[test]
fn a_held_generator_with_no_word_from_the_host_makes_nothing_and_loses_its_own_slots() {
    let guest = GuestGenerators::default();
    let own = [
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x700, A_PLACE),
        slot(1, 0, ANOTHER_PLACE),
        slot(GENERATOR_SLOT_STATE_FREED, 0, A_PLACE),
    ];
    let step = guest.step(A_GENERATOR, &own);
    assert!(step.adds.is_empty());
    assert_eq!(
        step.removals,
        vec![(0, SlotRemoval::NotTheHosts), (1, SlotRemoval::NotTheHosts)]
    );
    assert!(!step.finishes);
    assert_eq!(step.spawned, None);
}

#[test]
fn the_hosts_spawn_gets_a_slot_once_and_becomes_the_made_enemy() {
    let mut guest = GuestGenerators::default();
    let lines = guest.hear_spawn(spawn(1, A_PLACE));
    assert!(
        lines[0].starts_with("enemies: the host's generator made generator 0x5A0177C2 spawn 1")
    );
    let step = guest.step(A_GENERATOR, &[]);
    assert_eq!(step.adds, vec![spawn(1, A_PLACE)]);
    assert!(guest.step(ANOTHER_GENERATOR, &[]).adds.is_empty());
    guest.note_added(id(1));
    let waiting = [slot(0, 0, A_PLACE)];
    assert!(guest.step(A_GENERATOR, &waiting).adds.is_empty());
    assert!(guest.step(A_GENERATOR, &waiting).removals.is_empty());
    assert_eq!(
        guest.spawn_in_slot(A_GENERATOR, &waiting[0]),
        Some(spawn(1, A_PLACE))
    );
    assert_eq!(
        guest.spawn_in_slot(A_GENERATOR, &slot(0, 0, ANOTHER_PLACE)),
        None
    );
    assert!(guest.note_slots(A_GENERATOR, &waiting).is_empty());
    assert!(guest.made().is_empty());
    let made = [slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x4100, A_PLACE)];
    let lines = guest.note_slots(A_GENERATOR, &made);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("from the host's message"));
    assert_eq!(guest.made(), vec![(id(1), 0x4100)]);
    assert_eq!(guest.spawn_in_slot(A_GENERATOR, &made[0]), None);
    assert!(guest.step(A_GENERATOR, &made).removals.is_empty());
}

#[test]
fn two_spawns_at_one_place_get_a_slot_each() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.hear_spawn(spawn(2, A_PLACE));
    guest.note_added(id(1));
    guest.note_added(id(2));
    let slots = [
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x4100, A_PLACE),
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x4200, A_PLACE),
    ];
    assert_eq!(guest.note_slots(A_GENERATOR, &slots).len(), 2);
    assert_eq!(guest.made(), vec![(id(1), 0x4100), (id(2), 0x4200)]);
    assert!(guest.step(A_GENERATOR, &slots).removals.is_empty());
}

#[test]
fn a_made_enemys_slot_is_not_taken_by_a_waiting_spawn_at_the_same_place() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.hear_spawn(spawn(2, A_PLACE));
    guest.note_added(id(2));
    guest.note_slots(
        A_GENERATOR,
        &[slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x4200, A_PLACE)],
    );
    guest.note_added(id(1));
    let slots = [
        slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x4200, A_PLACE),
        slot(0, 0, A_PLACE),
    ];
    assert!(guest.step(A_GENERATOR, &slots).removals.is_empty());
    assert!(guest.note_slots(A_GENERATOR, &slots).is_empty());
    assert_eq!(guest.made(), vec![(id(2), 0x4200)]);
}

#[test]
fn a_spawn_the_game_could_not_make_is_a_mismatch_and_its_slot_goes() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.note_added(id(1));
    let failed = [slot(GENERATOR_SLOT_STATE_ENEMY_NOT_MADE, 0, A_PLACE)];
    let lines = guest.note_slots(A_GENERATOR, &failed);
    assert!(lines[0].starts_with("enemies: mismatch: this copy's game could not make"));
    let step = guest.step(A_GENERATOR, &failed);
    assert_eq!(step.removals, vec![(0, SlotRemoval::NotTheHosts)]);
    assert!(step.adds.is_empty());
}

#[test]
fn a_spawn_over_on_the_host_before_it_was_made_here_is_dropped() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    let line = guest.hear_gone(id(1), false).expect("a line");
    assert!(line.contains("was never made here"));
    assert!(guest.step(A_GENERATOR, &[]).adds.is_empty());
    assert_eq!(guest.spawns_held(), 0);
    assert_eq!(guest.hear_gone(id(1), false), None);
}

#[test]
fn a_spawn_over_on_the_host_while_its_slot_still_loads_loses_the_slot() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.note_added(id(1));
    assert_eq!(guest.hear_gone(id(1), false), None);
    let loading = [slot(1, 0, A_PLACE)];
    assert_eq!(
        guest.step(A_GENERATOR, &loading).removals,
        vec![(0, SlotRemoval::OverOnTheHostBeforeItWasMade)]
    );
    assert!(guest.note_slots(A_GENERATOR, &[]).is_empty());
    assert_eq!(guest.spawns_held(), 0);
}

#[test]
fn an_enemy_the_hosts_game_took_away_is_taken_away_here_and_then_forgotten() {
    let mut guest = guest_with_a_made_enemy(0x4100);
    assert!(guest.made_and_gone_on_the_host().is_empty());
    assert_eq!(guest.hear_gone(id(1), false), None);
    assert_eq!(guest.made_and_gone_on_the_host(), vec![(id(1), 0x4100)]);
    assert_eq!(guest.note_made_enemy_gone(id(1)), None);
    assert_eq!(guest.spawns_held(), 0);
}

#[test]
fn an_enemy_defeated_on_the_host_keeps_its_slot_until_its_own_defeat_has_played() {
    let mut guest = guest_with_a_made_enemy(0x4100);
    assert_eq!(guest.hear_gone(id(1), true), None);
    assert!(guest.made_and_gone_on_the_host().is_empty());
    let slots = [slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x4100, A_PLACE)];
    assert!(guest.step(A_GENERATOR, &slots).removals.is_empty());
    assert_eq!(guest.made(), vec![(id(1), 0x4100)]);
    assert_eq!(guest.note_made_enemy_gone(id(1)), None);
    assert_eq!(guest.spawns_held(), 0);
}

#[test]
fn an_enemy_gone_here_while_the_host_still_has_it_is_made_again() {
    let mut guest = guest_with_a_made_enemy(0x4100);
    let line = guest.note_made_enemy_gone(id(1)).expect("a line");
    assert!(line.contains("is made again"));
    assert!(guest.made().is_empty());
    assert_eq!(guest.step(A_GENERATOR, &[]).adds, vec![spawn(1, A_PLACE)]);
}

#[test]
fn a_slot_cleared_by_the_game_before_its_enemy_was_made_is_added_again() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.note_added(id(1));
    let lines = guest.note_slots(A_GENERATOR, &[]);
    assert!(lines[0].contains("gets a new one"));
    assert_eq!(guest.step(A_GENERATOR, &[]).adds, vec![spawn(1, A_PLACE)]);
}

#[test]
fn the_run_ends_here_when_it_has_ended_on_the_host_and_nothing_waits() {
    let mut guest = GuestGenerators::default();
    guest.hear_states(&[GeneratorState {
        generator: A_GENERATOR,
        phase: GeneratorPhase::Spawning,
        spawned: 3,
    }]);
    let step = guest.step(A_GENERATOR, &[]);
    assert!(!step.finishes);
    assert_eq!(step.spawned, Some(3));
    guest.hear_states(&[GeneratorState {
        generator: A_GENERATOR,
        phase: GeneratorPhase::Finished,
        spawned: 0,
    }]);
    assert!(guest.step(A_GENERATOR, &[]).finishes);
    assert!(!guest.step(ANOTHER_GENERATOR, &[]).finishes);
    guest.hear_spawn(spawn(4, A_PLACE));
    assert!(!guest.step(A_GENERATOR, &[]).finishes);
    assert_eq!(
        guest.host_state(A_GENERATOR).map(|state| state.phase),
        Some(GeneratorPhase::Finished)
    );
}

#[test]
fn a_new_link_forgets_the_hosts_spawns_so_their_slots_go() {
    let mut guest = guest_with_a_made_enemy(0x4100);
    guest.forget();
    let slots = [slot(GENERATOR_SLOT_STATE_ENEMY_MADE, 0x4100, A_PLACE)];
    assert_eq!(
        guest.step(A_GENERATOR, &slots).removals,
        vec![(0, SlotRemoval::NotTheHosts)]
    );
    assert!(guest.made().is_empty());
}

#[test]
fn the_ids_of_placed_and_generated_enemies_never_meet() {
    let placed = EnemyId::Placed {
        layout: A_GENERATOR,
        entry: 1,
    };
    assert_ne!(placed, id(1));
    assert!(id(1).is_generated());
    assert!(!placed.is_generated());
    assert_eq!(id(1).label(), "generator 0x5A0177C2 spawn 1");
    assert_eq!(placed.label(), "layout 0x5A0177C2 entry 0x1");
}

#[test]
fn a_spawn_this_copy_refused_is_not_tried_again() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.note_refused(id(1));
    assert!(guest.step(A_GENERATOR, &[]).adds.is_empty());
    assert!(guest.hear_gone(id(1), false).is_some());
    assert_eq!(guest.spawns_held(), 0);
}

#[test]
fn only_the_on_setting_lets_the_host_own_the_spawns() {
    use automata_game::enemies::EnemyShare;
    assert!(EnemyShare::On.lets_the_host_own_the_spawns());
    assert!(!EnemyShare::Watch.lets_the_host_own_the_spawns());
    assert!(!EnemyShare::Off.lets_the_host_own_the_spawns());
}

fn set_of_words(words: [i32; 9]) -> Vec<u8> {
    let mut set = vec![0_u8; GENERATOR_SET_LEN];
    for (index, word) in words.iter().enumerate() {
        set[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    set
}

#[test]
fn the_spawn_values_of_a_set_are_its_first_ones_unless_the_second_ones_are_asked_for_and_exist() {
    use automata_game::enemies::spawn_values_of_set;
    let set = set_of_words([0x2_0000, 2, -1, 7, 9, 1, 5, 6, 8]);
    assert_eq!(spawn_values_of_set(&set, false), Some((2, [2, 0, 7, 9])));
    assert_eq!(spawn_values_of_set(&set, true), Some((2, [5, 6, 8, 9])));
    let without_second = set_of_words([0x2_0000, 2, 3, 7, 9, 0, 5, 6, 8]);
    assert_eq!(
        spawn_values_of_set(&without_second, true),
        Some((2, [2, 3, 7, 9]))
    );
    assert_eq!(spawn_values_of_set(&set[..16], false), None);
}

#[test]
fn the_spawn_block_is_laid_out_as_the_games_own_spawn_fills_it() {
    use automata_game::enemies::spawn_block_bytes;
    let block = spawn_block_bytes(&A_PLACE, &A_FACING, &[2, 0, 7, 9], 14);
    let f32_at =
        |offset: usize| f32::from_le_bytes(block[offset..offset + 4].try_into().expect("4"));
    let i32_at =
        |offset: usize| i32::from_le_bytes(block[offset..offset + 4].try_into().expect("4"));
    assert_eq!(block.len(), 0x90);
    for (index, offset) in (0..0x40).step_by(4).enumerate() {
        let on_the_diagonal = index % 5 == 0;
        assert_eq!(f32_at(offset), if on_the_diagonal { 1.0 } else { 0.0 });
    }
    assert_eq!(f32_at(0x40), A_PLACE[0]);
    assert_eq!(f32_at(0x48), A_PLACE[2]);
    assert_eq!(f32_at(0x54), A_FACING[1]);
    assert_eq!(
        [f32_at(0x60), f32_at(0x64), f32_at(0x68), f32_at(0x6C)],
        [1.0; 4]
    );
    assert_eq!(
        [i32_at(0x70), i32_at(0x74), i32_at(0x78), i32_at(0x7C)],
        [2, 14, 0, 7]
    );
    assert_eq!(
        [i32_at(0x80), i32_at(0x84), i32_at(0x88), i32_at(0x8C)],
        [-1, 9, 1, -1]
    );
}

#[test]
fn a_spawn_whose_generator_this_copy_lacks_is_made_without_one_and_never_gets_a_slot() {
    use std::collections::BTreeSet;
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    let live_here: BTreeSet<u32> = [ANOTHER_GENERATOR].into();
    let has_it: BTreeSet<u32> = [A_GENERATOR].into();
    assert!(guest.spawns_without_a_generator_here(&has_it).is_empty());
    assert_eq!(
        guest.spawns_without_a_generator_here(&live_here),
        vec![spawn(1, A_PLACE)]
    );
    guest.note_load_asked_without_a_generator(id(1));
    assert!(guest.spawns_without_a_generator_here(&live_here).is_empty());
    assert_eq!(guest.loading_without_a_generator(), vec![spawn(1, A_PLACE)]);
    assert!(guest.step(A_GENERATOR, &[]).adds.is_empty());
    guest.note_made_without_a_generator(id(1), 0x5100);
    assert!(guest.loading_without_a_generator().is_empty());
    assert_eq!(guest.made(), vec![(id(1), 0x5100)]);
    assert_eq!(guest.made_without_a_generator(), vec![(id(1), 0x5100)]);
    assert!(guest.step(A_GENERATOR, &[]).adds.is_empty());
    assert!(guest.holds(id(1)));
    guest.hear_gone(id(1), false);
    assert_eq!(guest.made_and_gone_on_the_host(), vec![(id(1), 0x5100)]);
    guest.note_made_enemy_gone(id(1));
    assert!(!guest.holds(id(1)));
}

#[test]
fn a_spawn_over_on_the_host_while_it_loads_without_a_generator_is_dropped() {
    let mut guest = GuestGenerators::default();
    guest.hear_spawn(spawn(1, A_PLACE));
    guest.note_load_asked_without_a_generator(id(1));
    assert!(guest.hear_gone(id(1), true).is_some());
    assert!(!guest.holds(id(1)));
    assert!(
        guest_with_a_made_enemy(0x4100)
            .made_without_a_generator()
            .is_empty()
    );
}

#[test]
fn an_enemy_whose_defeat_was_heard_is_not_made_again_when_its_body_is_gone_here_first() {
    let mut guest = guest_with_a_made_enemy(0x4100);
    assert_eq!(guest.hear_gone(id(1), true), None);
    assert_eq!(guest.note_made_enemy_gone(id(1)), None);
    assert!(guest.step(A_GENERATOR, &[]).adds.is_empty());
    assert!(!guest.holds(id(1)));
}

#[test]
fn the_hosts_enemy_going_away_after_its_defeat_is_not_a_taking_away() {
    let mut guest = guest_with_a_made_enemy(0x4100);
    guest.hear_gone(id(1), true);
    guest.hear_gone(id(1), false);
    assert!(guest.made_and_gone_on_the_host().is_empty());
}
