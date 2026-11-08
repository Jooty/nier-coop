use rc_game::enemies::{CA_GAUGE_ENTRY_SIZE, CaGaugeEntry, CaGaugeEvent, CaGaugeRaw, CaGaugeWatch};

fn open_raw(handle_index: u32, value: f32, timer: f32) -> CaGaugeRaw {
    let mut raw = [0_u8; CA_GAUGE_ENTRY_SIZE];
    raw[0] = 1;
    raw[4..8].copy_from_slice(&handle_index.to_le_bytes());
    raw[8..12].copy_from_slice(&7_u32.to_le_bytes());
    raw[0x4C..0x50].copy_from_slice(&value.to_le_bytes());
    raw[0x50..0x54].copy_from_slice(&timer.to_le_bytes());
    raw
}

#[test]
fn an_entry_reads_its_handle_and_both_floats() {
    let entry = CaGaugeEntry::parse(&open_raw(12, 0.5, 2.25));
    assert!(entry.in_use);
    assert_eq!((entry.handle.index, entry.handle.generation), (12, 7));
    assert_eq!((entry.value, entry.timer), (0.5, 2.25));
}

#[test]
fn a_zeroed_entry_is_not_in_use() {
    let entry = CaGaugeEntry::parse(&[0; CA_GAUGE_ENTRY_SIZE]);
    assert!(!entry.in_use);
}

#[test]
fn raw_hex_has_two_digits_a_byte() {
    let entry = CaGaugeEntry::parse(&open_raw(1, 0.0, 0.0));
    assert_eq!(entry.raw_hex().len(), CA_GAUGE_ENTRY_SIZE * 2);
    assert!(entry.raw_hex().starts_with("01"));
}

#[test]
fn opening_changing_and_closing_each_report_once() {
    let mut watch = CaGaugeWatch::new();
    let open = CaGaugeEntry::parse(&open_raw(3, 0.0, 0.0));
    assert!(matches!(
        watch.observe(0, 5, &open),
        Some(CaGaugeEvent::Opened { index: 5, .. })
    ));
    assert_eq!(watch.observe(250, 5, &open), None);
    let moved = CaGaugeEntry::parse(&open_raw(3, 0.25, 0.0));
    assert!(matches!(
        watch.observe(500, 5, &moved),
        Some(CaGaugeEvent::Changed { index: 5, .. })
    ));
    let closed = CaGaugeEntry::parse(&[0; CA_GAUGE_ENTRY_SIZE]);
    assert_eq!(
        watch.observe(750, 5, &closed),
        Some(CaGaugeEvent::Closed { index: 5 })
    );
    assert_eq!(watch.observe(1000, 5, &closed), None);
}

#[test]
fn an_entry_logs_at_most_four_changes_a_second_and_the_rest_wait() {
    let mut watch = CaGaugeWatch::new();
    watch.observe(0, 0, &CaGaugeEntry::parse(&open_raw(1, 0.0, 0.0)));
    let logged = (1_u32..=9)
        .filter(|step| {
            let entry =
                CaGaugeEntry::parse(&open_raw(1, 0.0, f32::from(u16::try_from(*step).unwrap())));
            watch.observe(100 * u64::from(*step), 0, &entry).is_some()
        })
        .count();
    assert_eq!(logged, 4);
    let later = CaGaugeEntry::parse(&open_raw(1, 0.0, 99.0));
    assert!(watch.observe(1100, 0, &later).is_some());
}

#[test]
fn entries_are_watched_independently() {
    let mut watch = CaGaugeWatch::new();
    let open = CaGaugeEntry::parse(&open_raw(1, 0.0, 0.0));
    assert!(watch.observe(0, 1, &open).is_some());
    assert!(watch.observe(0, 2, &open).is_some());
}

#[test]
fn probe_on_line_names_both_addresses() {
    assert_eq!(
        rc_game::enemies::ca_gauge_probe_on_text(0x1000),
        "ca gauge: probe on, entries at 0x12dc150, manager 0x12dc140"
    );
}

#[test]
fn heartbeat_line_reports_counts_failures_and_manager_bytes() {
    let stats = rc_game::enemies::CaGaugeScanStats {
        in_use: 2,
        first_read_ok: true,
        read_failures: 3,
    };
    let mut raw = [0_u8; 32];
    raw[0] = 0xab;
    let text = rc_game::enemies::ca_gauge_heartbeat_text(&stats, Some(&raw));
    assert!(text.starts_with(
        "ca gauge: heartbeat, in use 2 of 64, first read ok, read failures 3, manager raw ab00"
    ));
    assert_eq!(text.len(), text.find("raw ").unwrap() + 4 + 64);
}

#[test]
fn heartbeat_line_says_failed_and_unreadable() {
    let stats = rc_game::enemies::CaGaugeScanStats {
        in_use: 0,
        first_read_ok: false,
        read_failures: 64,
    };
    assert_eq!(
        rc_game::enemies::ca_gauge_heartbeat_text(&stats, None),
        "ca gauge: heartbeat, in use 0 of 64, first read failed, read failures 64, manager raw unreadable"
    );
}

#[test]
fn interval_clock_fires_first_then_every_interval() {
    let mut clock = rc_game::enemies::IntervalClock::new();
    assert!(clock.due(100, 5000));
    assert!(!clock.due(4000, 5000));
    assert!(clock.due(5100, 5000));
    clock.reset();
    assert!(clock.due(5200, 5000));
}

#[test]
fn boss_hp_lines_have_the_planned_format() {
    assert_eq!(
        rc_game::enemies::boss_fight_hp_text(28800, 28800),
        "boss fight: hp 28800 of 28800 on my side"
    );
    assert_eq!(
        rc_game::enemies::boss_hp_text(100, 28800),
        "boss hp: 100 of 28800"
    );
}
