use replicant_game::story_progress::{
    ENDING_ACCOMPLISHED_HEADER_OFFSET, ENDING_CLEAR_BITS_HEADER_OFFSET,
    ENDING_COSTUMES_HEADER_OFFSET, EndingRecordsSession, FINISH_HOUR_ACCOMPLISHED_INDEX,
    HEADER_BYTES, HEADER_CHECKSUM_OFFSET, HeaderTooShort, SAVE_FILE_BYTES, apply_header_records,
    ending_records_gained, fix_header_checksum, header_checksum, merge_ending_records,
    read_ending_records,
};
use replicant_proto::{ENDING_ACCOMPLISHED_BYTES, EndingRecords};

const NAME_LOCK_OFFSET: usize = 0x08;
const SLOT_MASK_OFFSET: usize = 0x6C;
const FINISH_HOUR_OFFSET: usize = 0x197;
const FILE_MARKER_OFFSET: usize = 0;
const FILE_MARKER: u8 = 0x6E;

fn records(clear_bits: u32, costume: Option<usize>, accomplished: &[usize]) -> EndingRecords {
    let mut costumes = [0_u8; 7];
    if let Some(index) = costume {
        costumes[index] = 1;
    }
    let mut bytes = vec![0_u8; ENDING_ACCOMPLISHED_BYTES];
    for &index in accomplished {
        bytes[index] = 1;
    }
    EndingRecords {
        clear_bits,
        costumes,
        accomplished: bytes,
    }
}

fn file_with_guest_header() -> Vec<u8> {
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    file[FILE_MARKER_OFFSET] = FILE_MARKER;
    file[NAME_LOCK_OFFSET..NAME_LOCK_OFFSET + 4].copy_from_slice(b"GUES");
    file[SLOT_MASK_OFFSET] = 0x81;
    file[FINISH_HOUR_OFFSET] = 9;
    for (index, byte) in file[HEADER_BYTES..].iter_mut().enumerate() {
        *byte = (index % 251) as u8;
    }
    file
}

#[test]
fn merging_ors_the_bits_and_every_byte() {
    let merged = merge_ending_records(
        &records(0b0001, Some(0), &[0x16]),
        &records(0b1000, Some(2), &[0x17]),
    );
    assert_eq!(merged.clear_bits, 0b1001);
    assert_eq!(merged.costumes, [1, 0, 1, 0, 0, 0, 0]);
    assert_eq!(merged.accomplished[0x16], 1);
    assert_eq!(merged.accomplished[0x17], 1);
}

#[test]
fn merging_twice_gives_the_same_records() {
    let first = records(0b0011, Some(1), &[0x16]);
    let second = records(0b0100, None, &[0x17]);
    let once = merge_ending_records(&first, &second);
    assert_eq!(merge_ending_records(&once, &second), once);
}

#[test]
fn applying_records_keeps_the_guests_name_lock_slot_mask_and_hour() {
    let mut file = file_with_guest_header();
    let before = file.clone();
    apply_header_records(&mut file, &records(0b1000, Some(3), &[0x16])).unwrap();
    for offset in [NAME_LOCK_OFFSET, SLOT_MASK_OFFSET, FINISH_HOUR_OFFSET] {
        assert_eq!(file[offset], before[offset]);
    }
    assert_eq!(file[NAME_LOCK_OFFSET..NAME_LOCK_OFFSET + 4], *b"GUES");
    assert_eq!(file[FILE_MARKER_OFFSET], FILE_MARKER);
}

#[test]
fn applying_records_writes_them_and_fixes_the_checksum() {
    let mut file = file_with_guest_header();
    let wanted = records(0b1001, Some(3), &[0x16, 0x17]);
    assert_eq!(apply_header_records(&mut file, &wanted), Ok(true));
    assert_eq!(read_ending_records(&file), Some(wanted));
    let stored = u32::from_le_bytes(
        file[HEADER_CHECKSUM_OFFSET..HEADER_BYTES]
            .try_into()
            .unwrap(),
    );
    assert_eq!(Some(stored), header_checksum(&file));
    assert_eq!(file[FILE_MARKER_OFFSET], FILE_MARKER);
}

#[test]
fn applying_the_same_records_again_changes_nothing() {
    let mut file = file_with_guest_header();
    let wanted = records(0b1001, None, &[0x16]);
    apply_header_records(&mut file, &wanted).unwrap();
    let once = file.clone();
    assert_eq!(apply_header_records(&mut file, &wanted), Ok(false));
    assert_eq!(file, once);
}

#[test]
fn the_checksum_is_the_byte_sum_below_it() {
    let mut file = vec![0_u8; HEADER_BYTES];
    file[10] = 200;
    file[ENDING_CLEAR_BITS_HEADER_OFFSET] = 7;
    file[HEADER_CHECKSUM_OFFSET - 1] = 3;
    fix_header_checksum(&mut file).unwrap();
    assert_eq!(file[HEADER_CHECKSUM_OFFSET], 210);
    assert_eq!(&file[HEADER_CHECKSUM_OFFSET + 1..], &[0, 0, 0]);
}

#[test]
fn a_short_buffer_is_rejected_and_left_alone() {
    let mut short = vec![1_u8; HEADER_BYTES - 1];
    let before = short.clone();
    assert_eq!(
        apply_header_records(&mut short, &records(1, None, &[])),
        Err(HeaderTooShort)
    );
    assert_eq!(fix_header_checksum(&mut short), Err(HeaderTooShort));
    assert_eq!(short, before);
}

#[test]
fn slot_bytes_are_never_touched() {
    let mut file = file_with_guest_header();
    let before = file.clone();
    apply_header_records(&mut file, &records(0b1111, Some(6), &[0x16, 0x17])).unwrap();
    assert_eq!(file[HEADER_BYTES..], before[HEADER_BYTES..]);
}

#[test]
fn only_the_three_record_fields_and_the_checksum_change() {
    let mut file = file_with_guest_header();
    let before = file.clone();
    apply_header_records(&mut file, &records(0b1111, Some(6), &[0x16, 0x17])).unwrap();
    let in_field = |offset: usize| {
        (ENDING_CLEAR_BITS_HEADER_OFFSET..ENDING_CLEAR_BITS_HEADER_OFFSET + 4).contains(&offset)
            || (ENDING_COSTUMES_HEADER_OFFSET..ENDING_COSTUMES_HEADER_OFFSET + 7).contains(&offset)
            || (ENDING_ACCOMPLISHED_HEADER_OFFSET
                ..ENDING_ACCOMPLISHED_HEADER_OFFSET + ENDING_ACCOMPLISHED_BYTES)
                .contains(&offset)
            || (HEADER_CHECKSUM_OFFSET..HEADER_BYTES).contains(&offset)
    };
    for offset in 0..HEADER_BYTES {
        if !in_field(offset) {
            assert_eq!(file[offset], before[offset], "offset {offset:#x}");
        }
    }
}

#[test]
fn gained_records_are_what_is_new() {
    let gained = ending_records_gained(
        &records(0b0011, Some(0), &[0x16]),
        &records(0b0111, Some(0), &[0x16, 0x17]),
    );
    assert_eq!(gained.clear_bits, 0b0100);
    assert_eq!(gained.costumes, [0; 7]);
    assert_eq!(gained.accomplished[0x17], 1);
    assert_eq!(gained.accomplished[0x16], 0);
}

#[test]
fn unearned_host_records_never_reach_the_file() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0b0001, None, &[]));
    session.receive_host(records(0b0001, None, &[]), true);
    assert!(!session.receive_host(records(0b1001, Some(2), &[0x16]), false));
    assert_eq!(session.saved_records(), Some(records(0b0001, None, &[])));
}

#[test]
fn host_records_gained_together_are_earned() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0b0001, None, &[]));
    session.receive_host(records(0b0001, None, &[]), true);
    assert!(session.receive_host(records(0b1001, None, &[0x16]), true));
    assert_eq!(
        session.saved_records(),
        Some(records(0b1001, None, &[0x16]))
    );
}

#[test]
fn the_hosts_first_records_are_a_baseline_and_earn_nothing() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0, None, &[]));
    assert!(!session.receive_host(records(0b1111, Some(1), &[0x17]), true));
    assert_eq!(session.saved_records(), Some(records(0, None, &[])));
}

#[test]
fn live_records_during_the_session_are_the_hosts_or_the_guests_own() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0b0001, Some(0), &[]));
    session.receive_host(records(0b0100, None, &[0x17]), false);
    let live = session.live_target().unwrap();
    assert_eq!(live.clear_bits, 0b0101);
    assert_eq!(live.costumes[0], 1);
    assert_eq!(live.accomplished[0x17], 1);
}

#[test]
fn a_record_only_the_guest_set_is_earned_and_kept() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0b0001, None, &[]));
    session.receive_host(records(0b0001, None, &[]), true);
    assert!(session.observe_live(&records(0b0011, None, &[])));
    assert!(!session.observe_live(&records(0b0011, None, &[])));
    assert_eq!(session.saved_records(), Some(records(0b0011, None, &[])));
}

#[test]
fn a_host_record_seen_live_is_not_taken_for_the_guests_own() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0, None, &[]));
    session.receive_host(records(0, None, &[]), true);
    session.receive_host(records(0b1000, None, &[]), false);
    assert!(!session.observe_live(&records(0b1000, None, &[])));
    assert_eq!(session.saved_records(), Some(records(0, None, &[])));
}

#[test]
fn ending_the_session_hands_back_the_snapshot_or_earned_records() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0b0001, None, &[]));
    session.receive_host(records(0b0001, None, &[]), true);
    session.receive_host(records(0b0101, Some(2), &[]), true);
    session.receive_host(records(0b1101, Some(2), &[]), false);
    let expected = records(0b0101, Some(2), &[]);
    assert_eq!(session.end_session(), Some(expected.clone()));
    assert_eq!(session.end_session(), None);
    assert_eq!(session.saved_records(), Some(expected));
}

#[test]
fn records_earned_before_the_host_left_still_reach_a_later_save() {
    let mut session = EndingRecordsSession::default();
    session.arm(records(0, None, &[]));
    session.receive_host(records(0, None, &[]), true);
    session.receive_host(records(0b1000, None, &[0x16]), true);
    session.end_session();
    let mut file = file_with_guest_header();
    let written = read_ending_records(&file).unwrap();
    session.observe_live(&written);
    let target = session.saved_records().unwrap();
    apply_header_records(&mut file, &target).unwrap();
    assert_eq!(
        read_ending_records(&file),
        Some(records(0b1000, None, &[0x16]))
    );
}

#[test]
fn the_finish_hour_byte_never_travels_in_the_records() {
    let mut file = file_with_guest_header();
    let read = read_ending_records(&file).unwrap();
    assert_eq!(read.accomplished[FINISH_HOUR_ACCOMPLISHED_INDEX], 0);
    let mut sent = records(0, None, &[]);
    sent.accomplished[FINISH_HOUR_ACCOMPLISHED_INDEX] = 77;
    apply_header_records(&mut file, &sent).unwrap();
    assert_eq!(file[FINISH_HOUR_OFFSET], 9);
}
