use automata_game::address::{
    ENEMY_DAMAGE_FUNCTION, ENEMY_FLAT_DAMAGE_FUNCTION, ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION,
    ENTITY_LAYOUTS_LIST_HEAD_VALUE,
};
use automata_game::enemies::{
    Census, DefeatWatch, ENEMIES_IN_ONE_ROSTER_PART, ENEMY_HITS_IN_ONE_MESSAGE,
    ENTITY_LAYOUT_ENTITY_HANDLE_OFFSET, ENTITY_LAYOUT_ID_OFFSET, ENTITY_LAYOUT_LEN,
    ENTITY_LAYOUT_OBJECT_ID_OFFSET, ENTITY_LAYOUTS_HEADER_LEN, ENTITY_LAYOUTS_ID_OFFSET,
    ENTITY_LAYOUTS_ITEM_COUNT_OFFSET, ENTITY_LAYOUTS_ITEMS_POINTER_OFFSET,
    ENTITY_LAYOUTS_NEXT_POINTER_OFFSET, EnemyHit, EnemyHits, EnemyId, EnemyRosterPart, EnemyShare,
    EnemySighting, GuestHealthStep, HitLedger, HitOwner, HitSettlement, LayoutsHeader,
    MOST_ENEMIES_IN_ONE_ROSTER, MismatchWatch, Placement, ROSTER_MISMATCH_MUST_LAST_MS,
    RosterAssembly, RosterMismatch, RosterPacer, amount_in_the_hosts_scale, distinct_sightings,
    enemy_placements, guest_health_step, health_in_the_guests_scale, hit_owner, is_enemy_object_id,
    layouts_header, live_entity_slots, reads_as_an_enemys_health, roster_mismatches, roster_parts,
    settle_hit,
};
use automata_game::enemies::{
    MismatchPlaces, RosterHistory, UnplacedEnemyFacts, enemies_of_one_object_in_a_roster,
    unplaced_enemy_line,
};
use automata_game::exe::ExeVariant;
use automata_game::save_redirect::SaveRole;

const A_LAYOUT: u32 = 0x5A01_77C2;
const A_SMALL_ENEMY: u32 = 0x2_0000;
const ANOTHER_ENEMY: u32 = 0x2_0010;

fn id(entry: u32) -> EnemyId {
    EnemyId::Placed {
        layout: A_LAYOUT,
        entry,
    }
}

fn sighting(entry: u32, health: i32) -> EnemySighting {
    EnemySighting {
        id: id(entry),
        object_id: A_SMALL_ENEMY,
        health,
        max_health: 200,
    }
}

fn layout_entry(object_id: u32, handle: u32, entry_id: u32) -> Vec<u8> {
    let mut bytes = vec![0xEE_u8; ENTITY_LAYOUT_LEN];
    bytes[ENTITY_LAYOUT_OBJECT_ID_OFFSET..ENTITY_LAYOUT_OBJECT_ID_OFFSET + 4]
        .copy_from_slice(&object_id.to_le_bytes());
    bytes[ENTITY_LAYOUT_ENTITY_HANDLE_OFFSET..ENTITY_LAYOUT_ENTITY_HANDLE_OFFSET + 4]
        .copy_from_slice(&handle.to_le_bytes());
    bytes[ENTITY_LAYOUT_ID_OFFSET..ENTITY_LAYOUT_ID_OFFSET + 4]
        .copy_from_slice(&entry_id.to_le_bytes());
    bytes
}

#[test]
fn the_enemy_addresses_are_picked_per_exe() {
    assert_eq!(
        ENTITY_LAYOUTS_LIST_HEAD_VALUE.rva(ExeVariant::Main),
        0xFC9880
    );
    assert_eq!(
        ENTITY_LAYOUTS_LIST_HEAD_VALUE.rva(ExeVariant::Compat),
        0xFBD848
    );
    assert_eq!(ENEMY_DAMAGE_FUNCTION.rva(ExeVariant::Main), 0x66D2F0);
    assert_eq!(ENEMY_DAMAGE_FUNCTION.rva(ExeVariant::Compat), 0x665F60);
    assert_eq!(ENEMY_FLAT_DAMAGE_FUNCTION.rva(ExeVariant::Main), 0x66D760);
    assert_eq!(ENEMY_FLAT_DAMAGE_FUNCTION.rva(ExeVariant::Compat), 0x6663D0);
    assert_eq!(
        ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION.rva(ExeVariant::Main),
        0x5905D0
    );
    assert_eq!(
        ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION.rva(ExeVariant::Compat),
        0x589B60
    );
}

#[test]
fn a_layouts_header_gives_the_items_the_count_the_id_and_the_next_list() {
    let mut bytes = vec![0_u8; ENTITY_LAYOUTS_HEADER_LEN];
    bytes[ENTITY_LAYOUTS_ITEMS_POINTER_OFFSET..ENTITY_LAYOUTS_ITEMS_POINTER_OFFSET + 8]
        .copy_from_slice(&0x1_4000_2000_u64.to_le_bytes());
    bytes[ENTITY_LAYOUTS_ITEM_COUNT_OFFSET..ENTITY_LAYOUTS_ITEM_COUNT_OFFSET + 8]
        .copy_from_slice(&3_u64.to_le_bytes());
    bytes[ENTITY_LAYOUTS_ID_OFFSET..ENTITY_LAYOUTS_ID_OFFSET + 4]
        .copy_from_slice(&A_LAYOUT.to_le_bytes());
    bytes[ENTITY_LAYOUTS_NEXT_POINTER_OFFSET..ENTITY_LAYOUTS_NEXT_POINTER_OFFSET + 8]
        .copy_from_slice(&0x1_5000_0000_u64.to_le_bytes());
    assert_eq!(
        layouts_header(&bytes),
        Some(LayoutsHeader {
            items: 0x1_4000_2000,
            count: 3,
            id: A_LAYOUT,
            next: 0x1_5000_0000,
        })
    );
    assert_eq!(
        layouts_header(&bytes[..ENTITY_LAYOUTS_HEADER_LEN - 1]),
        None
    );
}

#[test]
fn only_entries_that_place_an_enemy_and_hold_a_handle_are_placements() {
    let mut items = layout_entry(A_SMALL_ENEMY, 0x0001_2300, 7);
    items.extend(layout_entry(0x5_0001, 0x0001_2400, 8));
    items.extend(layout_entry(A_SMALL_ENEMY, 0, 9));
    items.extend(layout_entry(A_SMALL_ENEMY, u32::MAX, 10));
    items.extend(layout_entry(ANOTHER_ENEMY, 0x0002_0100, 11));
    items.extend(&layout_entry(A_SMALL_ENEMY, 0x0003_0100, 12)[..ENTITY_LAYOUT_LEN - 1]);
    assert_eq!(
        enemy_placements(A_LAYOUT, &items),
        vec![
            Placement {
                id: id(7),
                entry_id: 7,
                object_id: A_SMALL_ENEMY,
                handle: 0x0001_2300,
            },
            Placement {
                id: id(11),
                entry_id: 11,
                object_id: ANOTHER_ENEMY,
                handle: 0x0002_0100,
            },
        ]
    );
}

#[test]
fn the_enemy_object_ids_are_the_two_xxxx_range() {
    assert!(is_enemy_object_id(0x2_0000));
    assert!(is_enemy_object_id(0x2_FFFF));
    assert!(!is_enemy_object_id(0x1_0000));
    assert!(!is_enemy_object_id(0x3_0000));
    assert!(!is_enemy_object_id(0xA_2000));
}

#[test]
fn two_live_enemies_with_one_id_are_both_left_out() {
    let all = [sighting(2, 50), sighting(1, 200), sighting(2, 70)];
    let (distinct, doubled) = distinct_sightings(&all);
    assert_eq!(distinct, vec![sighting(1, 200)]);
    assert_eq!(doubled, vec![id(2)]);
}

#[test]
fn a_census_says_each_enemy_once_when_seen_and_once_when_gone() {
    let mut census = Census::default();
    let first = census.note(0, &[sighting(1, 200), sighting(2, 200)], &[]);
    assert_eq!(first.len(), 1);
    assert!(first[0].starts_with("enemies: seen "));
    assert!(first[0].contains("entry 0x1") && first[0].contains("entry 0x2"));
    assert!(
        census
            .note(0, &[sighting(1, 150), sighting(2, 200)], &[])
            .is_empty()
    );
    let after = census.note(0, &[sighting(2, 200)], &[]);
    assert_eq!(after.len(), 1);
    assert!(after[0].starts_with("enemies: gone "));
    assert!(after[0].contains("entry 0x1") && after[0].contains("health 150/200"));
    assert_eq!(census.known(), vec![sighting(2, 200)]);
}

#[test]
fn a_census_names_six_enemies_and_counts_the_rest() {
    let mut census = Census::default();
    let many: Vec<EnemySighting> = (1..=9).map(|entry| sighting(entry, 200)).collect();
    let lines = census.note(0, &many, &[]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].ends_with("; and 3 more"));
}

#[test]
fn a_census_says_a_doubled_id_once_while_it_stays_doubled() {
    let mut census = Census::default();
    let lines = census.note(0, &[], &[id(4)]);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("share one id"));
    assert!(census.note(0, &[], &[id(4)]).is_empty());
}

#[test]
fn a_census_stops_logging_at_its_minutes_budget_and_says_how_many_it_held_back() {
    let mut census = Census::default();
    let mut all_lines = Vec::new();
    for round in 0..200_u32 {
        let shown = if round % 2 == 0 {
            vec![sighting(1, 200)]
        } else {
            Vec::new()
        };
        all_lines.extend(census.note(1_000, &shown, &[]));
    }
    assert_eq!(
        all_lines.len(),
        automata_game::enemies::CENSUS_LINES_LOGGED_PER_MINUTE as usize
    );
    let next = census.note(61_000, &[sighting(1, 200)], &[]);
    assert_eq!(next.len(), 2);
    assert_eq!(
        next[0],
        "enemies: seen and gone lines: 140 more lines of this kind held back until the next minute"
    );
}

#[test]
fn rosters_mismatch_on_an_enemy_only_one_copy_has_and_on_another_object() {
    let here = [sighting(1, 200), sighting(2, 200), sighting(3, 200)];
    let other_object = EnemySighting {
        object_id: ANOTHER_ENEMY,
        ..sighting(3, 200)
    };
    let there = [sighting(2, 90), other_object, sighting(4, 200)];
    assert_eq!(
        roster_mismatches(&here, &there),
        vec![
            RosterMismatch::OnlyHere(sighting(1, 200)),
            RosterMismatch::OtherObject {
                here: sighting(3, 200),
                there: other_object,
            },
            RosterMismatch::OnlyThere(sighting(4, 200)),
        ]
    );
    assert!(roster_mismatches(&here, &here).is_empty());
}

#[test]
fn a_mismatch_is_said_once_after_it_has_lasted_and_once_when_it_is_over() {
    let mut watch = MismatchWatch::default();
    let mismatch = [RosterMismatch::OnlyHere(sighting(1, 200))];
    assert!(watch.note(1_000, &mismatch).is_empty());
    assert!(
        watch
            .note(1_000 + ROSTER_MISMATCH_MUST_LAST_MS - 1, &mismatch)
            .is_empty()
    );
    let said = watch.note(1_000 + ROSTER_MISMATCH_MUST_LAST_MS, &mismatch);
    assert_eq!(said.len(), 1);
    assert!(said[0].starts_with("enemies: mismatch: only this copy has "));
    assert!(watch.note(60_000, &mismatch).is_empty());
    let over = watch.note(60_100, &[]);
    assert_eq!(over.len(), 1);
    assert!(over[0].ends_with("is over"));
}

#[test]
fn a_mismatch_that_ends_before_it_has_lasted_is_never_said() {
    let mut watch = MismatchWatch::default();
    let mismatch = [RosterMismatch::OnlyThere(sighting(1, 200))];
    assert!(watch.note(0, &mismatch).is_empty());
    assert!(watch.note(500, &[]).is_empty());
    assert!(
        watch
            .note(ROSTER_MISMATCH_MUST_LAST_MS, &mismatch)
            .is_empty()
    );
}

#[test]
fn a_roster_goes_out_in_parts_and_comes_back_whole() {
    let enemies: Vec<EnemySighting> = (0..(ENEMIES_IN_ONE_ROSTER_PART as u32 * 2 + 3))
        .map(|entry| sighting(entry, 200))
        .collect();
    let parts = roster_parts(9, &enemies);
    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(EnemyRosterPart::is_valid));
    assert_eq!(
        parts.iter().map(|part| part.last).collect::<Vec<_>>(),
        vec![false, false, true]
    );
    let mut assembly = RosterAssembly::default();
    let mut whole = None;
    for part in parts {
        whole = assembly.receive(part);
    }
    assert_eq!(whole, Some(enemies));
}

#[test]
fn an_empty_roster_is_one_closing_part() {
    let parts = roster_parts(1, &[]);
    assert_eq!(
        parts,
        vec![EnemyRosterPart {
            round: 1,
            last: true,
            enemies: Vec::new(),
        }]
    );
    assert_eq!(
        RosterAssembly::default().receive(parts[0].clone()),
        Some(Vec::new())
    );
}

#[test]
fn a_part_of_a_new_round_drops_the_unfinished_one() {
    let mut assembly = RosterAssembly::default();
    let old = EnemyRosterPart {
        round: 1,
        last: false,
        enemies: vec![sighting(1, 200)],
    };
    let new = EnemyRosterPart {
        round: 2,
        last: true,
        enemies: vec![sighting(2, 200)],
    };
    assert_eq!(assembly.receive(old), None);
    assert_eq!(assembly.receive(new), Some(vec![sighting(2, 200)]));
}

#[test]
fn a_roster_past_the_limit_is_cut_on_both_ends() {
    let enemies: Vec<EnemySighting> = (0..(MOST_ENEMIES_IN_ONE_ROSTER as u32 + 40))
        .map(|entry| sighting(entry, 200))
        .collect();
    let sent: usize = roster_parts(1, &enemies)
        .iter()
        .map(|part| part.enemies.len())
        .sum();
    assert_eq!(sent, MOST_ENEMIES_IN_ONE_ROSTER);
    let mut assembly = RosterAssembly::default();
    let mut whole = None;
    for (index, chunk) in enemies.chunks(ENEMIES_IN_ONE_ROSTER_PART).enumerate() {
        whole = assembly.receive(EnemyRosterPart {
            round: 5,
            last: (index + 1) * ENEMIES_IN_ONE_ROSTER_PART >= enemies.len(),
            enemies: chunk.to_vec(),
        });
    }
    assert_eq!(
        whole.map(|roster| roster.len()),
        Some(MOST_ENEMIES_IN_ONE_ROSTER)
    );
}

#[test]
fn a_roster_part_or_a_hit_list_past_its_limits_is_not_valid() {
    let too_many = EnemyRosterPart {
        round: 1,
        last: true,
        enemies: vec![sighting(1, 200); ENEMIES_IN_ONE_ROSTER_PART + 1],
    };
    assert!(!too_many.is_valid());
    let negative = EnemyRosterPart {
        round: 1,
        last: true,
        enemies: vec![sighting(1, -5)],
    };
    assert!(!negative.is_valid());
    let hit = EnemyHit {
        id: id(1),
        amount: 30,
        max_health: 200,
    };
    assert!(EnemyHits { hits: vec![hit] }.is_valid());
    assert!(
        !EnemyHits {
            hits: vec![hit; ENEMY_HITS_IN_ONE_MESSAGE + 1]
        }
        .is_valid()
    );
    assert!(
        !EnemyHits {
            hits: vec![EnemyHit {
                max_health: 0,
                ..hit
            }]
        }
        .is_valid()
    );
    assert!(
        !EnemyHits {
            hits: vec![EnemyHit {
                amount: u32::MAX,
                ..hit
            }]
        }
        .is_valid()
    );
}

#[test]
fn a_changed_roster_is_sent_soon_and_an_unchanged_one_once_a_second() {
    let mut pacer = RosterPacer::default();
    let roster = [sighting(1, 200)];
    assert!(pacer.is_due(0, &roster));
    let first_round = pacer.parts_to_send(0, &roster)[0].round;
    assert!(!pacer.is_due(999, &roster));
    assert!(pacer.is_due(1_000, &roster));
    let hurt = [sighting(1, 150)];
    assert!(!pacer.is_due(99, &hurt));
    assert!(pacer.is_due(100, &hurt));
    assert_ne!(pacer.parts_to_send(100, &hurt)[0].round, first_round);
    pacer.forget();
    assert!(pacer.is_due(101, &hurt));
}

#[test]
fn a_hit_by_the_body_the_other_player_drives_is_the_partners() {
    assert_eq!(hit_owner(Some(0x1200), Some(0x1200)), HitOwner::PartnerBody);
    assert_eq!(hit_owner(Some(0x1300), Some(0x1200)), HitOwner::LocalPlayer);
    assert_eq!(hit_owner(None, Some(0x1200)), HitOwner::LocalPlayer);
    assert_eq!(hit_owner(Some(0x1200), None), HitOwner::LocalPlayer);
}

#[test]
fn the_partner_bodys_hit_is_taken_back_on_both_copies() {
    for role in [SaveRole::Host, SaveRole::Guest] {
        assert_eq!(
            settle_hit(role, HitOwner::PartnerBody, 120, 0),
            HitSettlement {
                health_written: Some(120),
                amount_reported: 0,
            }
        );
    }
}

#[test]
fn the_hosts_own_hit_stands_as_the_game_applied_it() {
    assert_eq!(
        settle_hit(SaveRole::Host, HitOwner::LocalPlayer, 120, 0),
        HitSettlement::default()
    );
}

#[test]
fn the_guests_own_hit_is_reported_and_never_takes_the_last_health() {
    assert_eq!(
        settle_hit(SaveRole::Guest, HitOwner::LocalPlayer, 120, 90),
        HitSettlement {
            health_written: None,
            amount_reported: 30,
        }
    );
    assert_eq!(
        settle_hit(SaveRole::Guest, HitOwner::LocalPlayer, 20, 0),
        HitSettlement {
            health_written: Some(1),
            amount_reported: 20,
        }
    );
}

#[test]
fn a_hit_that_took_nothing_or_fell_on_a_defeated_enemy_changes_nothing() {
    assert_eq!(
        settle_hit(SaveRole::Guest, HitOwner::LocalPlayer, 120, 120),
        HitSettlement::default()
    );
    assert_eq!(
        settle_hit(SaveRole::Guest, HitOwner::PartnerBody, 0, -3),
        HitSettlement::default()
    );
}

#[test]
fn an_amount_keeps_its_share_of_full_health_across_copies() {
    assert_eq!(amount_in_the_hosts_scale(50, 200, 200), 50);
    assert_eq!(amount_in_the_hosts_scale(50, 200, 400), 100);
    assert_eq!(amount_in_the_hosts_scale(1, 1_000, 10), 1);
    assert_eq!(amount_in_the_hosts_scale(0, 200, 400), 0);
    assert_eq!(amount_in_the_hosts_scale(50, 0, 400), 50);
}

#[test]
fn the_hosts_health_keeps_its_share_and_a_living_enemy_keeps_one_health() {
    assert_eq!(health_in_the_guests_scale(150, 200, 200), 150);
    assert_eq!(health_in_the_guests_scale(150, 200, 400), 300);
    assert_eq!(health_in_the_guests_scale(1, 1_000, 10), 1);
    assert_eq!(health_in_the_guests_scale(0, 200, 400), 0);
    assert_eq!(health_in_the_guests_scale(500, 200, 200), 200);
}

#[test]
fn the_guest_writes_the_hosts_health_onto_its_own_enemy() {
    let local = sighting(1, 200);
    assert_eq!(
        guest_health_step(&local, Some(&sighting(1, 140)), false, None),
        GuestHealthStep::Write(140)
    );
    assert_eq!(
        guest_health_step(&local, Some(&sighting(1, 200)), false, None),
        GuestHealthStep::Leave
    );
    assert_eq!(
        guest_health_step(&local, None, false, None),
        GuestHealthStep::Leave
    );
}

#[test]
fn the_guest_defeats_its_enemy_when_the_hosts_is_defeated() {
    let local = sighting(1, 200);
    assert_eq!(
        guest_health_step(&local, Some(&sighting(1, 0)), false, None),
        GuestHealthStep::Defeat
    );
    assert_eq!(
        guest_health_step(&local, None, true, None),
        GuestHealthStep::Defeat
    );
    assert_eq!(
        guest_health_step(&sighting(1, 0), None, true, None),
        GuestHealthStep::Leave
    );
}

#[test]
fn the_guest_leaves_an_enemy_that_is_another_object_on_the_host() {
    let host = EnemySighting {
        object_id: ANOTHER_ENEMY,
        ..sighting(1, 0)
    };
    assert_eq!(
        guest_health_step(&sighting(1, 200), Some(&host), false, None),
        GuestHealthStep::Leave
    );
}

#[test]
fn right_after_its_own_hit_the_guest_does_not_raise_health_back_to_the_hosts() {
    let local = sighting(1, 150);
    let host = sighting(1, 200);
    assert_eq!(
        guest_health_step(&local, Some(&host), false, Some(100)),
        GuestHealthStep::Leave
    );
    assert_eq!(
        guest_health_step(&local, Some(&host), false, Some(500)),
        GuestHealthStep::Write(200)
    );
    assert_eq!(
        guest_health_step(&local, Some(&sighting(1, 100)), false, Some(100)),
        GuestHealthStep::Write(100)
    );
}

#[test]
fn the_guests_hits_on_one_enemy_are_summed_until_they_are_sent() {
    let mut ledger = HitLedger::default();
    ledger.add(id(1), 30, 200);
    ledger.add(id(1), 20, 200);
    ledger.add(id(2), 5, 80);
    assert_eq!(
        ledger.take_messages(),
        vec![EnemyHits {
            hits: vec![
                EnemyHit {
                    id: id(1),
                    amount: 50,
                    max_health: 200,
                },
                EnemyHit {
                    id: id(2),
                    amount: 5,
                    max_health: 80,
                },
            ],
        }]
    );
    assert!(ledger.take_messages().is_empty());
}

#[test]
fn many_hit_enemies_go_out_in_valid_messages() {
    let mut ledger = HitLedger::default();
    for entry in 0..(ENEMY_HITS_IN_ONE_MESSAGE as u32 + 2) {
        ledger.add(id(entry), 1, 200);
    }
    let messages = ledger.take_messages();
    assert_eq!(messages.len(), 2);
    assert!(messages.iter().all(EnemyHits::is_valid));
}

#[test]
fn the_host_announces_a_defeat_once_and_again_after_the_enemy_lived_again() {
    let mut watch = DefeatWatch::default();
    assert!(watch.newly_defeated(&[sighting(1, 200)]).is_empty());
    assert_eq!(watch.newly_defeated(&[sighting(1, 0)]), vec![id(1)]);
    assert!(watch.newly_defeated(&[sighting(1, 0)]).is_empty());
    assert!(watch.newly_defeated(&[]).is_empty());
    assert!(watch.holds(id(1)));
    assert!(watch.newly_defeated(&[sighting(1, 200)]).is_empty());
    assert!(!watch.holds(id(1)));
    assert_eq!(watch.newly_defeated(&[sighting(1, -2)]), vec![id(1)]);
}

#[test]
fn a_heard_defeat_is_held_until_the_host_shows_the_enemy_alive() {
    let mut heard = DefeatWatch::default();
    heard.hear(id(3));
    assert!(heard.holds(id(3)));
    heard.forget_the_ones_alive_in(&[sighting(3, 0)]);
    assert!(heard.holds(id(3)));
    heard.forget_the_ones_alive_in(&[sighting(3, 10)]);
    assert!(!heard.holds(id(3)));
    heard.hear(id(4));
    heard.forget();
    assert!(heard.all().is_empty());
}

#[test]
fn the_share_setting_reads_its_three_words() {
    assert_eq!(EnemyShare::from_text("on"), Some(EnemyShare::On));
    assert_eq!(EnemyShare::from_text("watch"), Some(EnemyShare::Watch));
    assert_eq!(EnemyShare::from_text("off"), Some(EnemyShare::Off));
    assert_eq!(EnemyShare::from_text("yes"), None);
    assert_eq!(EnemyShare::default(), EnemyShare::On);
    assert!(EnemyShare::On.lists_enemies() && EnemyShare::On.shares_health());
    assert!(EnemyShare::Watch.lists_enemies() && !EnemyShare::Watch.shares_health());
    assert!(!EnemyShare::Off.lists_enemies() && !EnemyShare::Off.shares_health());
}

#[test]
fn an_object_whose_health_fields_hold_something_else_does_not_read_as_an_enemy() {
    assert!(reads_as_an_enemys_health(200, 200));
    assert!(reads_as_an_enemys_health(0, 200));
    assert!(reads_as_an_enemys_health(-30, 200));
    assert!(!reads_as_an_enemys_health(1_135_167_437, 1_065_353_216));
    assert!(!reads_as_an_enemys_health(0, 0));
    assert!(!reads_as_an_enemys_health(300, 200));
}

#[test]
fn only_used_slots_of_the_entity_table_are_live_entities() {
    let mut table = Vec::new();
    for (handle, entity) in [
        (0x0001_0100_u32, 0x1_4000_0000_u64),
        (0, 0x1_4000_1000),
        (0x0002_0200, 0),
    ] {
        table.extend(handle.to_le_bytes());
        table.extend([0_u8; 4]);
        table.extend(entity.to_le_bytes());
    }
    table.extend([0xFF_u8; 5]);
    assert_eq!(
        live_entity_slots(&table),
        vec![(0x0001_0100, 0x1_4000_0000)]
    );
}

fn some_places() -> MismatchPlaces {
    MismatchPlaces {
        here: "0x12 AA".to_owned(),
        there: "0x13 BB".to_owned(),
    }
}

#[test]
fn an_unplaced_enemy_line_names_object_set_class_position_place_and_the_other_list() {
    let facts = UnplacedEnemyFacts {
        object_id: 0x2_0010,
        name: "en".to_owned(),
        set_id: 0x41,
        set_word: 0x7,
        vftable_rva: Some(0x123_4560),
        position: Some([1.0, 2.5, -3.0]),
        local_story_place: "0x12 AA".to_owned(),
        same_object_in_the_other_roster: Some(2),
    };
    assert_eq!(
        unplaced_enemy_line(&facts),
        "enemies: no layout entry or generator holds object 0x20010 named \"en\": set id 0x41, set word 0x7, class vftable rva 0x1234560 (no class name is known), stands at 1.0,2.5,-3.0, this copy's story place 0x12 AA; the other copy's list holds 2 enemies of this object id (its list tells no positions, so within 5 units cannot be said)"
    );
    let unread = UnplacedEnemyFacts {
        vftable_rva: None,
        position: None,
        same_object_in_the_other_roster: None,
        ..facts
    };
    let line = unplaced_enemy_line(&unread);
    assert!(line.contains("class not read"));
    assert!(line.contains("stands at not read"));
    assert!(line.ends_with("no list from the other copy yet"));
}

#[test]
fn enemies_of_one_object_are_counted_in_a_roster() {
    let roster = [sighting(1, 100), sighting(2, 100)];
    assert_eq!(
        enemies_of_one_object_in_a_roster(Some(&roster), A_SMALL_ENEMY),
        Some(2)
    );
    assert_eq!(
        enemies_of_one_object_in_a_roster(Some(&roster), ANOTHER_ENEMY),
        Some(0)
    );
    assert_eq!(enemies_of_one_object_in_a_roster(None, A_SMALL_ENEMY), None);
}

#[test]
fn a_mismatch_detail_says_how_long_it_is_held_and_whether_the_other_copy_reported_it() {
    let mut history = RosterHistory::default();
    let mine = sighting(1, 100);
    history.note_here(1_000, &[mine]);
    history.note_here(4_000, &[mine]);
    let only_here = RosterMismatch::OnlyHere(mine);
    assert_eq!(
        history.detail_of(5_000, &only_here, &some_places()),
        " [this copy has had it for 4000 ms; story places: here 0x12 AA, other copy 0x13 BB; the other copy has never reported it]"
    );
    history.note_other(&[mine]);
    assert!(
        history
            .detail_of(5_000, &only_here, &some_places())
            .ends_with("the other copy reported it before and it is gone there]")
    );
    history.note_here(6_000, &[]);
    let only_there = RosterMismatch::OnlyThere(mine);
    let detail = history.detail_of(7_000, &only_there, &some_places());
    assert!(detail.contains("this copy had it before and it is gone here"));
    assert!(detail.ends_with("the other copy reports it now]"));
    let stranger = RosterMismatch::OnlyThere(sighting(9, 100));
    assert!(
        history
            .detail_of(7_000, &stranger, &some_places())
            .contains("this copy has never had it")
    );
}

#[test]
fn a_detailed_mismatch_line_ends_with_its_detail() {
    let mut watch = MismatchWatch::default();
    let mismatch = [RosterMismatch::OnlyHere(sighting(1, 200))];
    watch.note_detailed(0, &mismatch, &|_| " [more]".to_owned());
    let said = watch.note_detailed(ROSTER_MISMATCH_MUST_LAST_MS, &mismatch, &|_| {
        " [more]".to_owned()
    });
    assert_eq!(said.len(), 1);
    assert!(said[0].starts_with("enemies: mismatch: only this copy has "));
    assert!(said[0].ends_with(" [more]"));
}
