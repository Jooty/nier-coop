use replicant_game::cube_probe::{
    ActorLink, GrabEvent, GrabWatch, LinkKind, LinkTarget, ascii_runs, distance_between_points,
    find_links, grab_link_text, read_block_lossy,
};

use crate::fake_memory::FakeMemory;

fn seconds(count: u64) -> u64 {
    count * 1000
}

#[test]
fn a_grab_starts_on_the_grab_start_motion_and_ends_on_the_release_motion() {
    let mut watch = GrabWatch::default();
    assert_eq!(watch.observe(0, Some("NIER013_idle")), GrabEvent::Idle);
    assert_eq!(
        watch.observe(10, Some("NIER013_boxGrab_st")),
        GrabEvent::Started
    );
    assert!(watch.holding());
    assert_eq!(
        watch.observe(20, Some("NIER013_boxPull_lp")),
        GrabEvent::Idle
    );
    assert_eq!(
        watch.observe(600, Some("NIER013_boxPull_lp")),
        GrabEvent::Holding
    );
    assert_eq!(
        watch.observe(700, Some("NIER013_boxRelease")),
        GrabEvent::Released
    );
    assert!(!watch.holding());
}

#[test]
fn the_partner_puppet_motion_names_without_a_prefix_count_too() {
    let mut watch = GrabWatch::default();
    assert_eq!(watch.observe(0, Some("boxGrab_st")), GrabEvent::Started);
    assert_eq!(watch.observe(5, Some("boxRelease")), GrabEvent::Released);
}

#[test]
fn holding_is_reported_every_half_second() {
    let mut watch = GrabWatch::default();
    watch.observe(0, Some("boxGrab_st"));
    let events: Vec<GrabEvent> = (1..=12)
        .map(|step| watch.observe(step * 100, Some("boxPull_lp")))
        .collect();
    let holding = events
        .iter()
        .filter(|event| **event == GrabEvent::Holding)
        .count();
    assert_eq!(holding, 2);
}

#[test]
fn a_hold_that_leaves_the_box_motions_for_too_long_gives_up() {
    let mut watch = GrabWatch::default();
    watch.observe(0, Some("boxGrab_st"));
    assert_eq!(watch.observe(100, Some("NIER013_idle")), GrabEvent::Idle);
    assert_eq!(
        watch.observe(100 + seconds(2), Some("NIER013_idle")),
        GrabEvent::GaveUp
    );
}

#[test]
fn a_short_gap_in_the_box_motions_does_not_end_the_hold() {
    let mut watch = GrabWatch::default();
    watch.observe(0, Some("boxGrab_st"));
    watch.observe(100, None);
    watch.observe(200, Some("boxPull_lp"));
    watch.observe(1500, None);
    assert!(watch.holding());
}

#[test]
fn a_pointer_to_a_nearby_actor_inside_the_player_is_a_grab_link() {
    let mut block = vec![0_u8; 0x40];
    block[0x18..0x20].copy_from_slice(&0x1234_5678_u64.to_le_bytes());
    block[0x30..0x38].copy_from_slice(&0xAAAA_u64.to_le_bytes());
    let targets = [
        LinkTarget {
            actor: 0x1234_5678,
            handle: 0x77,
        },
        LinkTarget {
            actor: 0x9999,
            handle: 0xAAAA,
        },
    ];
    assert_eq!(
        find_links(&block, &targets),
        vec![
            ActorLink {
                offset: 0x18,
                actor: 0x1234_5678,
                kind: LinkKind::Pointer
            },
            ActorLink {
                offset: 0x30,
                actor: 0x9999,
                kind: LinkKind::Handle
            },
        ]
    );
}

#[test]
fn no_link_is_reported_for_zero_and_the_text_says_none_were_found() {
    let targets = [LinkTarget {
        actor: 0,
        handle: 0,
    }];
    assert!(find_links(&[0_u8; 64], &targets).is_empty());
    assert!(grab_link_text("host", "local", 0x2000, &[]).contains("none"));
}

#[test]
fn readable_text_runs_are_found_with_their_offsets() {
    let mut bytes = vec![0_u8; 0x20];
    bytes[4..4 + 8].copy_from_slice(b"BOX_01aa");
    bytes[0x10..0x13].copy_from_slice(b"ab\x01");
    assert_eq!(ascii_runs(&bytes, 5), vec![(4, "BOX_01aa".to_owned())]);
}

#[test]
fn unreadable_chunks_of_the_player_block_read_as_zero() {
    let mut memory = FakeMemory::new();
    memory.put(0x1000, &[7_u8; 8]);
    let block = read_block_lossy(&memory, 0x1000, 0x800);
    assert_eq!(block.len(), 0x800);
    assert!(block.iter().all(|byte| *byte == 0));
}

#[test]
fn distance_is_three_dimensional() {
    let distance = distance_between_points([0.0, 0.0, 0.0], [3.0, 4.0, 12.0]);
    assert!((distance - 13.0).abs() < 1e-4);
}
