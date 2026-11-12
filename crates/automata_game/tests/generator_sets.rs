use std::collections::BTreeSet;

use automata_game::enemies::{
    GENERATOR_SET_MISMATCH_MUST_LAST_MS, GeneratorGroupWord, GeneratorSetComparison,
    LIVE_GENERATOR_IDS_IN_ONE_PART, LIVE_GENERATORS_ARE_TOLD_AGAIN_AFTER_MS, LiveGenerators,
    LiveGeneratorsAssembly, LiveGeneratorsPacer, MOST_LIVE_GENERATOR_PARTS, component_in_map,
    live_generator_parts,
};
use automata_game::start::{ROUTE_FLAGS_BY_HASH, route_flag_hash_held, route_flag_label};

const ROUTE_A: u32 = 0x5830_82BB;
const ROUTE_B: u32 = 0xC139_D301;

fn ids(values: &[u32]) -> BTreeSet<u32> {
    values.iter().copied().collect()
}

fn crc32(text: &str) -> u32 {
    let mut crc = u32::MAX;
    for byte in text.bytes() {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn comparison_with_the_hosts(route_hash: u32, hosts: &[u32]) -> GeneratorSetComparison {
    let mut comparison = GeneratorSetComparison::default();
    comparison.hear_the_hosts(route_hash, ids(hosts));
    comparison
}

#[test]
fn the_route_flag_hashes_are_the_crc32_of_their_names() {
    for (hash, name) in ROUTE_FLAGS_BY_HASH {
        assert_eq!(hash, crc32(name), "{name}");
    }
}

#[test]
fn the_route_held_is_the_first_route_flag_the_game_has() {
    assert_eq!(route_flag_hash_held(|hash| hash == ROUTE_B), ROUTE_B);
    assert_eq!(route_flag_hash_held(|_| false), 0);
    assert_eq!(route_flag_label(ROUTE_A), "Route/A");
    assert_eq!(route_flag_label(0), "no route flag");
}

#[test]
fn a_large_set_goes_in_numbered_parts_and_an_empty_one_in_one_empty_part() {
    let many: BTreeSet<u32> = (1..=130).collect();
    let parts = live_generator_parts(ROUTE_A, &many);
    assert_eq!(parts.len(), 3);
    assert_eq!(
        parts.iter().map(|part| part.ids.len()).collect::<Vec<_>>(),
        [
            LIVE_GENERATOR_IDS_IN_ONE_PART,
            LIVE_GENERATOR_IDS_IN_ONE_PART,
            2
        ]
    );
    assert!(
        parts
            .iter()
            .enumerate()
            .all(|(index, part)| usize::from(part.part) == index && part.parts == 3)
    );
    assert!(parts.iter().all(LiveGenerators::is_valid));
    let empty = live_generator_parts(ROUTE_A, &BTreeSet::new());
    assert_eq!(empty.len(), 1);
    assert!(empty[0].ids.is_empty() && empty[0].is_valid());
}

#[test]
fn a_part_past_the_count_or_too_full_is_refused() {
    let part = LiveGenerators {
        route_hash: ROUTE_A,
        part: 2,
        parts: 2,
        ids: Vec::new(),
    };
    assert!(!part.is_valid());
    let full = LiveGenerators {
        part: 0,
        parts: 1,
        ids: vec![1; LIVE_GENERATOR_IDS_IN_ONE_PART + 1],
        ..part.clone()
    };
    assert!(!full.is_valid());
    let too_many_parts = LiveGenerators {
        part: 0,
        parts: MOST_LIVE_GENERATOR_PARTS + 1,
        ..part
    };
    assert!(!too_many_parts.is_valid());
}

#[test]
fn the_guest_puts_the_parts_together_and_a_new_telling_starts_over() {
    let many: BTreeSet<u32> = (1..=100).collect();
    let parts = live_generator_parts(ROUTE_A, &many);
    let mut assembly = LiveGeneratorsAssembly::default();
    assert_eq!(assembly.receive(parts[0].clone()), None);
    assert_eq!(assembly.receive(parts[0].clone()), None);
    assert_eq!(assembly.receive(parts[1].clone()), Some((ROUTE_A, many)));
    let few = live_generator_parts(ROUTE_B, &ids(&[7]));
    assert_eq!(assembly.receive(few[0].clone()), Some((ROUTE_B, ids(&[7]))));
}

#[test]
fn the_host_tells_its_set_when_it_changes_and_again_after_a_while() {
    let mut pacer = LiveGeneratorsPacer::default();
    assert_eq!(pacer.parts_to_send(0, ROUTE_A, &ids(&[1, 2])).len(), 1);
    assert!(
        pacer
            .parts_to_send(1_000, ROUTE_A, &ids(&[1, 2]))
            .is_empty()
    );
    assert_eq!(pacer.parts_to_send(1_500, ROUTE_A, &ids(&[1])).len(), 1);
    assert_eq!(pacer.parts_to_send(1_600, ROUTE_B, &ids(&[1])).len(), 1);
    assert!(
        pacer
            .parts_to_send(
                1_600 + LIVE_GENERATORS_ARE_TOLD_AGAIN_AFTER_MS - 1,
                ROUTE_B,
                &ids(&[1])
            )
            .is_empty()
    );
    assert_eq!(
        pacer
            .parts_to_send(
                1_600 + LIVE_GENERATORS_ARE_TOLD_AGAIN_AFTER_MS,
                ROUTE_B,
                &ids(&[1])
            )
            .len(),
        1
    );
    pacer.forget();
    assert_eq!(pacer.parts_to_send(7_000, ROUTE_B, &ids(&[1])).len(), 1);
}

#[test]
fn the_same_set_is_said_at_once_and_only_once() {
    let mut comparison = comparison_with_the_hosts(ROUTE_A, &[1, 2, 3]);
    assert_eq!(
        comparison.note(0, ROUTE_A, &ids(&[1, 2, 3])).as_deref(),
        Some(
            "generators: same set as the host: 3 live generators on both copies (Route/A on both)"
        )
    );
    assert_eq!(comparison.note(1_000, ROUTE_A, &ids(&[1, 2, 3])), None);
}

#[test]
fn a_difference_is_said_only_once_it_has_lasted_and_names_both_sides() {
    let mut comparison = comparison_with_the_hosts(ROUTE_A, &[0x10, 0x20, 0x30]);
    let own = ids(&[0x20, 0x40]);
    assert_eq!(comparison.note(0, ROUTE_B, &own), None);
    assert_eq!(
        comparison.note(GENERATOR_SET_MISMATCH_MUST_LAST_MS - 1, ROUTE_B, &own),
        None
    );
    assert_eq!(
        comparison
            .note(GENERATOR_SET_MISMATCH_MUST_LAST_MS, ROUTE_B, &own)
            .as_deref(),
        Some(
            "generators: mismatch: the copies hold different generators (Route/A on the host, Route/B here): 2 only on the host (10 30), 1 only here (40)"
        )
    );
    assert_eq!(comparison.note(10_000, ROUTE_B, &own), None);
}

#[test]
fn a_difference_that_changes_starts_its_wait_again() {
    let mut comparison = comparison_with_the_hosts(ROUTE_A, &[1, 2]);
    assert_eq!(comparison.note(0, ROUTE_A, &ids(&[1])), None);
    assert_eq!(comparison.note(2_000, ROUTE_A, &ids(&[2])), None);
    assert_eq!(
        comparison.note(
            2_000 + GENERATOR_SET_MISMATCH_MUST_LAST_MS - 1,
            ROUTE_A,
            &ids(&[2])
        ),
        None
    );
    assert!(
        comparison
            .note(
                2_000 + GENERATOR_SET_MISMATCH_MUST_LAST_MS,
                ROUTE_A,
                &ids(&[2])
            )
            .is_some()
    );
}

#[test]
fn nothing_is_compared_without_the_hosts_set_or_while_either_copy_holds_none() {
    let mut unheard = GeneratorSetComparison::default();
    assert_eq!(unheard.note(0, ROUTE_A, &ids(&[1])), None);
    let mut comparison = comparison_with_the_hosts(ROUTE_A, &[1]);
    assert_eq!(comparison.note(0, ROUTE_A, &BTreeSet::new()), None);
    let mut empty_host = comparison_with_the_hosts(ROUTE_A, &[]);
    assert_eq!(empty_host.note(10_000, ROUTE_A, &ids(&[1])), None);
}

#[test]
fn after_a_relink_the_result_is_said_again() {
    let mut comparison = comparison_with_the_hosts(ROUTE_A, &[1]);
    assert!(comparison.note(0, ROUTE_A, &ids(&[1])).is_some());
    comparison.forget();
    assert_eq!(comparison.note(100, ROUTE_A, &ids(&[1])), None);
    comparison.hear_the_hosts(ROUTE_A, ids(&[1]));
    assert!(comparison.note(200, ROUTE_A, &ids(&[1])).is_some());
}

fn map_entry(kind: i32, value: u64) -> Vec<u8> {
    let mut entry = kind.to_le_bytes().to_vec();
    entry.extend([0; 4]);
    entry.extend(value.to_le_bytes());
    entry
}

#[test]
fn the_group_component_is_found_by_its_kind_and_an_empty_value_counts_as_none() {
    let mut entries = map_entry(3, 0x1111);
    entries.extend(map_entry(7, 0x2222_0000));
    entries.extend(map_entry(9, 0));
    assert_eq!(component_in_map(&entries, 7), Some(0x2222_0000));
    assert_eq!(component_in_map(&entries, 9), None);
    assert_eq!(component_in_map(&entries, 4), None);
    assert_eq!(component_in_map(&entries[..20], 7), None);
}

#[test]
fn a_group_word_says_what_was_found() {
    assert_eq!(GeneratorGroupWord::Word(0x1A).label(), "0x1A");
    assert_eq!(
        GeneratorGroupWord::NoComponent.label(),
        "none (the enemy has no such component)"
    );
}
