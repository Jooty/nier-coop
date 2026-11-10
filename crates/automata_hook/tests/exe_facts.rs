use automata_game::exe::{COMPAT_EXE, ExeFacts, NEW_MAIN_EXE};
use automata_hook::exe_facts::{facts_from_image, gate_report_line, sha256_hex};
use nc_hook::build_gate::PeError;

const PE_OFFSET: usize = 0x80;

fn image_with_timestamp(time_date_stamp: u32) -> Vec<u8> {
    let mut image = vec![0_u8; 0x1000];
    image[..2].copy_from_slice(b"MZ");
    image[0x3C..0x40].copy_from_slice(&(PE_OFFSET as u32).to_le_bytes());
    image[PE_OFFSET..PE_OFFSET + 4].copy_from_slice(b"PE\0\0");
    image[PE_OFFSET + 8..PE_OFFSET + 12].copy_from_slice(&time_date_stamp.to_le_bytes());
    image
}

#[test]
fn sha256_of_a_known_input_is_lowercase_hex() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn facts_carry_size_timestamp_and_hash_of_the_image() {
    let image = image_with_timestamp(0x6AC8_9519);
    let facts = facts_from_image(&image).expect("a valid header");
    assert_eq!(facts.file_size, image.len() as u64);
    assert_eq!(facts.time_date_stamp, 0x6AC8_9519);
    assert_eq!(facts.sha256_hex, sha256_hex(&image));
}

#[test]
fn a_file_that_is_not_a_pe_image_is_refused() {
    assert_eq!(facts_from_image(&[0_u8; 0x400]), Err(PeError::NotDosImage));
}

#[test]
fn a_synthetic_image_is_not_a_known_exe() {
    let facts = facts_from_image(&image_with_timestamp(NEW_MAIN_EXE.time_date_stamp))
        .expect("a valid header");
    let line = gate_report_line(&facts);
    assert!(line.starts_with("unknown exe"), "{line}");
    assert!(line.contains("nothing installed"), "{line}");
}

#[test]
fn a_known_exe_reports_its_variant() {
    let main = ExeFacts {
        file_size: NEW_MAIN_EXE.file_size,
        time_date_stamp: NEW_MAIN_EXE.time_date_stamp,
        sha256_hex: NEW_MAIN_EXE.sha256_hex.to_owned(),
    };
    assert!(gate_report_line(&main).starts_with("exe variant: main"));
    let compat = ExeFacts {
        file_size: COMPAT_EXE.file_size,
        time_date_stamp: COMPAT_EXE.time_date_stamp,
        sha256_hex: COMPAT_EXE.sha256_hex.to_owned(),
    };
    assert!(gate_report_line(&compat).starts_with("exe variant: compat"));
}
